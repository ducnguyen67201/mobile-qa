//! Real PostgreSQL coverage for immutable nonsecret model registration.
mod support;

use mobile_qa::services::model_registry;
use mobile_qa::services::{execution_store::*, runs, scheduler, test_definitions, worker_auth};
use mobile_qa_contracts::automation::DirectCommand;
use mobile_qa_contracts::execution::{
    ActionKind, ClaimRequest, Driver, ExecutionProfile, JobState, QueueReason, RunResponse,
};
use mobile_qa_contracts::model_registry::*;
use mobile_qa_contracts::task_sessions::{OpenPhoneRequest, PhoneSession};
use support::*;

fn definition(revision: u32) -> ModelDefinition {
    ModelDefinition {
        reference: ModelReference {
            key: "synthetic.registry".into(),
            revision,
        },
        display_name: "Synthetic registry model".into(),
        provider: ModelProvider::OpenAi,
        provider_model: "synthetic-model".into(),
        capabilities: vec![ModelCapability::MinitapNavigation],
    }
}

#[test]
fn multi_reference_capabilities_are_bounded_and_exact() {
    let model = definition(1).resolve();
    let capabilities = WorkerModelCapabilities {
        model: None,
        models: vec![model.reference.clone()],
        providers: vec![ModelProvider::OpenAi],
    };
    assert!(capabilities.validate(6).is_ok());
    assert!(model_registry::worker_matches(&model, Some(&capabilities)));
    assert!(capabilities.validate(5).is_err());
    let mut duplicate = capabilities.clone();
    duplicate.models.push(model.reference.clone());
    assert!(duplicate.validate(6).is_err());
    let mut mismatch = model;
    mismatch.reference.revision = 2;
    assert!(!model_registry::worker_matches(
        &mismatch,
        Some(&capabilities)
    ));
    assert_eq!(
        model_registry::eligible_models(Some(&capabilities))
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn assignment_revisions_change_only_future_resolution() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App,_,_>(|server,ctx|async move {
        let owner=login(&server,&ctx).await;
        let mut input=create_input(owner.org);
        input.android_package="ai.mobileqa.demo".into();
        let app=apps::create(&ctx,owner.user,input).await.unwrap().id;
        let profile=ExecutionProfile{execution_context:None,id:Uuid::new_v4(),name:"Assignment fixture".into(),driver:Driver::Minitap,package:"ai.mobileqa.demo".into(),adapter:"demo_persistence_v1".into(),device_identity:Uuid::new_v4().to_string(),image:"test".into(),model:None,qualified:true,qualification_reference:"synthetic only".into(),max_apk_bytes:104857600};
        test_definitions::register_profile(&ctx,owner.user,app,profile.clone()).await.unwrap();
        let worker=Uuid::new_v4();
        worker_auth::register(&ctx,owner.user,app,worker,profile.id,&loco_rs::hash::random_string(64)).await.unwrap();
        let mut first=definition(1);
        let mut second=definition(2);
        let mut third=definition(3);
        third.capabilities=vec![ModelCapability::StructuredAuthoring];
        let key=format!("synthetic.assignment.{}",Uuid::new_v4());
        first.reference.key=key.clone();
        second.reference.key=key;
        third.reference.key=second.reference.key.clone();
        model_registry::register(&ctx.db,owner.user,&first).await.unwrap();
        model_registry::register(&ctx.db,owner.user,&second).await.unwrap();
        model_registry::register(&ctx.db,owner.user,&third).await.unwrap();
        let caps=WorkerModelCapabilities{model:None,models:vec![first.reference.clone(),second.reference.clone(),third.reference.clone()],providers:vec![ModelProvider::OpenAi]};
        model_registry::advertise_phone(&ctx.db,worker,6,Some(&caps)).await.unwrap();
        model_registry::advertise_execution(&ctx.db,worker,6,Some(&caps)).await.unwrap();
        let mut assignment=ModelAssignment{app_id:app,profile_id:profile.id,purpose:ModelPurpose::Navigation,revision:1,reference:first.reference.clone(),max_calls:20};
        model_registry::stage_assignment(&ctx,owner.user,&assignment).await.unwrap();
        model_registry::transition_assignment(&ctx,owner.user,&assignment,ModelAssignmentState::Active).await.unwrap();
        let frozen=model_registry::active_assignment(&ctx.db,app,profile.id,ModelPurpose::Navigation).await.unwrap().unwrap();
        assert_eq!(frozen.0.reference,first.reference);
        assignment.revision=2;
        assignment.reference=second.reference.clone();
        model_registry::stage_assignment(&ctx,owner.user,&assignment).await.unwrap();
        model_registry::transition_assignment(&ctx,owner.user,&assignment,ModelAssignmentState::Active).await.unwrap();
        let current=model_registry::active_assignment(&ctx.db,app,profile.id,ModelPurpose::Navigation).await.unwrap().unwrap();
        assert_eq!(current.0.reference,second.reference);
        assert_eq!(frozen.0.reference,first.reference);
        assert_eq!(frozen.1,1);
        assert_eq!(current.1,2);
        let authoring=ModelAssignment{app_id:app,profile_id:profile.id,purpose:ModelPurpose::StructuredAuthoring,revision:1,reference:third.reference.clone(),max_calls:10};
        model_registry::stage_assignment(&ctx,owner.user,&authoring).await.unwrap();
        let authoring_worker=Uuid::new_v4();
        worker_auth::register(&ctx,owner.user,app,authoring_worker,profile.id,&loco_rs::hash::random_string(64)).await.unwrap();
        let navigation_only=WorkerModelCapabilities{model:None,models:vec![second.reference.clone()],providers:vec![ModelProvider::OpenAi]};
        let authoring_only=WorkerModelCapabilities{model:None,models:vec![third.reference.clone()],providers:vec![ModelProvider::OpenAi]};
        model_registry::advertise_phone(&ctx.db,worker,6,Some(&navigation_only)).await.unwrap();
        model_registry::advertise_phone(&ctx.db,authoring_worker,6,Some(&authoring_only)).await.unwrap();
        assert!(model_registry::transition_assignment(&ctx,owner.user,&authoring,ModelAssignmentState::Active).await.is_err());
        model_registry::advertise_phone(&ctx.db,worker,6,Some(&caps)).await.unwrap();
        model_registry::transition_assignment(&ctx,owner.user,&authoring,ModelAssignmentState::Active).await.unwrap();
        let bytes=fixture("execution");
        let upload=new_upload(&server,&owner,app,&bytes).await;
        transfer(&server,&owner,app,upload.id,bytes).await.assert_status_ok();
        let build=owner.write(server.post(&format!("/api/apps/{app}/build-uploads/{}/complete",upload.id))).await.json::<mobile_qa_contracts::browser::BuildResponse>();
        let mut direct_fixture:RunResponse=serde_json::from_str(include_str!("fixtures/execution/comparison.json")).unwrap();
        let mut direct_profile=direct_fixture.manifest.profile.clone();
        direct_profile.id=Uuid::new_v4();
        direct_profile.device_identity=Uuid::new_v4().to_string();
        direct_profile.model=None;
        let direct_context=direct_profile.execution_context.as_mut().unwrap();
        direct_context.qualified_profile_id=direct_profile.id;
        let starting_check=&mut direct_context.starting_checks[0];
        starting_check.id="ready".into();
        starting_check.checkpoint_id="preflight".into();
        starting_check.text_filter.clear();
        starting_check.expected="true".into();
        starting_check.prerequisite_check_ids.clear();
        starting_check.required=true;
        test_definitions::register_profile(&ctx,owner.user,app,direct_profile.clone()).await.unwrap();
        let direct_worker=Uuid::new_v4();
        worker_auth::register(&ctx,owner.user,app,direct_worker,direct_profile.id,&loco_rs::hash::random_string(64)).await.unwrap();
        direct_fixture.manifest.app_id=app;
        direct_fixture.manifest.build_id=build.id;
        direct_fixture.manifest.profile=direct_profile;
        direct_fixture.manifest.cases[0].case.actions[0].kind=ActionKind::Direct;
        direct_fixture.manifest.cases[0].case.actions[0].instruction.clear();
        direct_fixture.manifest.cases[0].case.actions[0].command=Some(DirectCommand::Back{});
        let direct_run=Uuid::new_v4();
        exec(&ctx.db,"INSERT INTO execution_runs(id,app_id,creator_id,build_id,plan_id,idempotency_key,fingerprint,manifest) VALUES($1,$2,$3,$4,NULL,$5,$6,$7)",vec![direct_run.into(),app.into(),owner.user.into(),build.id.into(),"protocol-gate".into(),"protocol-gate".into(),json(&direct_fixture.manifest).unwrap().into()]).await.unwrap();
        exec(&ctx.db,"INSERT INTO execution_attempts(id,run_id,case_index) VALUES($1,$2,0)",vec![Uuid::new_v4().into(),direct_run.into()]).await.unwrap();
        model_registry::advertise_execution(&ctx.db,direct_worker,3,None).await.unwrap();
        let waiting=runs::detail(&ctx.db,direct_run).await.unwrap().queue_status.unwrap();
        assert_eq!(waiting.reason,QueueReason::WorkerUpgradeRequired);
        assert!(waiting.last_compatible_worker_at.is_none());
        model_registry::advertise_execution(&ctx.db,direct_worker,4,None).await.unwrap();
        let ready=runs::detail(&ctx.db,direct_run).await.unwrap().queue_status.unwrap();
        assert_eq!(ready.reason,QueueReason::AwaitingWorkerClaim);
        assert!(ready.last_compatible_worker_at.is_some());
        let phone=owner.write(server.post(&format!("/api/apps/{app}/phones"))).json(&OpenPhoneRequest{id:Uuid::new_v4(),build_id:Some(build.id),profile_id:Some(profile.id)}).await;
        phone.assert_status_ok();
        let phone=phone.json::<PhoneSession>();
        assert_eq!(phone.resolved_model.unwrap().reference,second.reference);
        assert_eq!(phone.authoring_model.unwrap().reference,third.reference);
        assert_eq!(phone.authoring_assignment_revision,Some(1));
        let mut fixture:RunResponse=serde_json::from_str(include_str!("fixtures/execution/comparison.json")).unwrap();
        fixture.manifest.app_id=app;
        fixture.manifest.build_id=build.id;
        fixture.manifest.profile=profile.clone();
        fixture.manifest.source=None;
        fixture.manifest.model_assignment_revision=Some(1);
        fixture.manifest.resolved_model=Some(frozen.0);
        let older=Uuid::new_v4();
        exec(&ctx.db,"INSERT INTO execution_runs(id,app_id,creator_id,build_id,plan_id,idempotency_key,fingerprint,manifest) VALUES($1,$2,$3,$4,NULL,$5,$6,$7)",vec![older.into(),app.into(),owner.user.into(),build.id.into(),"older-model".into(),"older-model".into(),json(&fixture.manifest).unwrap().into()]).await.unwrap();
        exec(&ctx.db,"INSERT INTO execution_attempts(id,run_id,case_index) VALUES($1,$2,0)",vec![Uuid::new_v4().into(),older.into()]).await.unwrap();
        fixture.manifest.model_assignment_revision=Some(2);
        fixture.manifest.resolved_model=Some(current.0);
        let newer=Uuid::new_v4();
        exec(&ctx.db,"INSERT INTO execution_runs(id,app_id,creator_id,build_id,plan_id,idempotency_key,fingerprint,manifest) VALUES($1,$2,$3,$4,NULL,$5,$6,$7)",vec![newer.into(),app.into(),owner.user.into(),build.id.into(),"newer-model".into(),"newer-model".into(),json(&fixture.manifest).unwrap().into()]).await.unwrap();
        exec(&ctx.db,"INSERT INTO execution_attempts(id,run_id,case_index) VALUES($1,$2,0)",vec![Uuid::new_v4().into(),newer.into()]).await.unwrap();
        let only_second=WorkerModelCapabilities{model:None,models:vec![second.reference.clone()],providers:vec![ModelProvider::OpenAi]};
        model_registry::advertise_execution(&ctx.db,worker,6,Some(&only_second)).await.unwrap();
        assert_eq!(runs::detail(&ctx.db,older).await.unwrap().queue_status.unwrap().reason,QueueReason::ModelUnavailable);
        exec(&ctx.db,"UPDATE execution_workers SET execution_last_seen_at=now()-interval '5 minutes' WHERE id=$1",vec![worker.into()]).await.unwrap();
        assert_eq!(runs::detail(&ctx.db,older).await.unwrap().queue_status.unwrap().reason,QueueReason::WorkerOffline);
        let lease=scheduler::claim(&ctx,&worker_auth::Worker{id:worker,app_id:app,profile_id:profile.id},ClaimRequest{version:6,claim_id:Uuid::new_v4(),profile_id:profile.id,model_capabilities:Some(only_second)}).await.unwrap().lease.unwrap();
        assert_eq!(lease.run_id,newer);
        assert_eq!(runs::detail(&ctx.db,older).await.unwrap().queue_status.unwrap().reason,QueueReason::CapacityBusy);
        exec(&ctx.db,"UPDATE execution_attempts SET state='recovery_required' WHERE id=$1",vec![lease.attempt_id.into()]).await.unwrap();
        assert_eq!(runs::detail(&ctx.db,older).await.unwrap().queue_status.unwrap().reason,QueueReason::DeviceRecoveryRequired);
    }).await;
}

#[tokio::test]
async fn recovery_queue_links_only_to_a_run_in_the_same_app() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let mut input = create_input(owner.org);
        input.android_package = "ai.mobileqa.demo".into();
        let app = apps::create(&ctx, owner.user, input).await.unwrap().id;
        let bytes = fixture("execution");
        let upload = new_upload(&server, &owner, app, &bytes).await;
        transfer(&server, &owner, app, upload.id, bytes)
            .await
            .assert_status_ok();
        let build = owner
            .write(server.post(&format!("/api/apps/{app}/build-uploads/{}/complete", upload.id)))
            .await
            .json::<BuildResponse>();

        let mut run_fixture: RunResponse =
            serde_json::from_str(include_str!("fixtures/execution/comparison.json")).unwrap();
        run_fixture.manifest.app_id = app;
        run_fixture.manifest.build_id = build.id;
        run_fixture.manifest.profile.device_identity = Uuid::new_v4().to_string();
        let device = run_fixture.manifest.profile.device_identity.clone();
        let blocker = Uuid::new_v4();
        exec(&ctx.db, "INSERT INTO execution_runs(id,app_id,creator_id,build_id,plan_id,idempotency_key,fingerprint,manifest) VALUES($1,$2,$3,$4,NULL,$5,$6,$7)", vec![blocker.into(),app.into(),owner.user.into(),build.id.into(),"blocker".into(),"blocker".into(),json(&run_fixture.manifest).unwrap().into()]).await.unwrap();
        let attempt = Uuid::new_v4();
        exec(&ctx.db, "INSERT INTO execution_attempts(id,run_id,case_index,state,cleanup) VALUES($1,$2,0,'recovery_required','quarantined')", vec![attempt.into(),blocker.into()]).await.unwrap();
        exec(&ctx.db, "INSERT INTO execution_reservations(resource,attempt_id) VALUES($1,$2)", vec![format!("device:{device}").into(),attempt.into()]).await.unwrap();

        let waiting = Uuid::new_v4();
        exec(&ctx.db, "INSERT INTO execution_runs(id,app_id,creator_id,build_id,plan_id,idempotency_key,fingerprint,manifest) VALUES($1,$2,$3,$4,NULL,$5,$6,$7)", vec![waiting.into(),app.into(),owner.user.into(),build.id.into(),"waiting".into(),"waiting".into(),json(&run_fixture.manifest).unwrap().into()]).await.unwrap();
        exec(&ctx.db, "INSERT INTO execution_attempts(id,run_id,case_index) VALUES($1,$2,0)", vec![Uuid::new_v4().into(),waiting.into()]).await.unwrap();
        let same_app = owner.read(server.get(&format!("/api/runs/{waiting}"))).await.json::<RunResponse>().queue_status.unwrap();
        assert_eq!(same_app.reason, QueueReason::DeviceRecoveryRequired);
        assert_eq!(same_app.blocking_run_id, Some(blocker));

        let cancelled = owner.write(server.post(&format!("/api/runs/{blocker}/cancel"))).await.json::<RunResponse>();
        assert!(cancelled.cancel_requested);
        assert_eq!(cancelled.state, JobState::RecoveryRequired);
        let persisted = owner.read(server.get(&format!("/api/runs/{blocker}"))).await.json::<RunResponse>();
        assert!(persisted.cancel_requested);
        assert_eq!(persisted.state, JobState::RecoveryRequired);

        // A shared device can block a different app without disclosing its run ID.
        let foreign = login(&server, &ctx).await;
        let mut foreign_input = create_input(foreign.org);
        foreign_input.android_package = "ai.mobileqa.demo".into();
        let foreign_app = apps::create(&ctx, foreign.user, foreign_input).await.unwrap().id;
        let bytes = fixture("execution");
        let upload = new_upload(&server, &foreign, foreign_app, &bytes).await;
        transfer(&server, &foreign, foreign_app, upload.id, bytes)
            .await
            .assert_status_ok();
        let foreign_build = foreign.write(server.post(&format!("/api/apps/{foreign_app}/build-uploads/{}/complete", upload.id))).await.json::<BuildResponse>();
        run_fixture.manifest.app_id = foreign_app;
        run_fixture.manifest.build_id = foreign_build.id;
        let foreign_run = Uuid::new_v4();
        exec(&ctx.db, "INSERT INTO execution_runs(id,app_id,creator_id,build_id,plan_id,idempotency_key,fingerprint,manifest) VALUES($1,$2,$3,$4,NULL,$5,$6,$7)", vec![foreign_run.into(),foreign_app.into(),foreign.user.into(),foreign_build.id.into(),"foreign-device".into(),"foreign-device".into(),json(&run_fixture.manifest).unwrap().into()]).await.unwrap();
        exec(&ctx.db, "INSERT INTO execution_attempts(id,run_id,case_index) VALUES($1,$2,0)", vec![Uuid::new_v4().into(),foreign_run.into()]).await.unwrap();
        let cross_app = foreign.read(server.get(&format!("/api/runs/{foreign_run}"))).await.json::<RunResponse>().queue_status.unwrap();
        assert_eq!(cross_app.reason, QueueReason::DeviceRecoveryRequired);
        assert_eq!(cross_app.blocking_run_id, None);
    }).await;
}

#[tokio::test]
async fn registry_is_immutable_resolvable_and_retirable() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|_server, ctx| async move {
        let mut model = definition(1);
        model.reference.key = format!("synthetic.registry.{}", Uuid::new_v4());
        let actor = uuid::Uuid::new_v4();
        let first = model_registry::register(&ctx.db, actor, &model)
            .await
            .unwrap();
        assert_eq!(first.reference, model.reference);
        assert_eq!(
            model_registry::register(&ctx.db, actor, &model)
                .await
                .unwrap(),
            first
        );
        let mut mutation = model.clone();
        mutation.provider_model = "changed".into();
        assert!(model_registry::register(&ctx.db, actor, &mutation)
            .await
            .is_err());
        assert!(model_registry::resolve_for_new_work(
            &ctx.db,
            Some(&ModelBinding::Registered(model.reference.clone())),
            &[ModelCapability::MinitapNavigation],
        )
        .await
        .unwrap()
        .is_some());
        model_registry::retire(&ctx.db, &model.reference)
            .await
            .unwrap();
        assert!(model_registry::resolve_for_new_work(
            &ctx.db,
            Some(&ModelBinding::Registered(model.reference)),
            &[],
        )
        .await
        .is_err());
    })
    .await;
}
