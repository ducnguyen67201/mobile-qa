//! Real routes/database. Synthetic worker evidence is not Android acceptance.
mod support;
use mobile_qa::services::{
    execution_store::*, runs, scheduler, test_definitions as defs, verification, worker_auth,
};
use mobile_qa_contracts::execution::*;
use sea_orm::TransactionTrait;
use support::*;

fn definition() -> TestDefinition {
    serde_json::from_str(include_str!(
        "../../../contracts/fixtures/execution/persistence-case.json"
    ))
    .unwrap()
}
async fn saved_definition(
    ctx: &AppContext,
    owner: &Login,
    app: Uuid,
    d: TestDefinition,
) -> DefinitionResponse {
    let row = defs::import(
        ctx,
        owner.user,
        DefinitionImport {
            app_id: app,
            definition: d,
        },
    )
    .await
    .unwrap();
    defs::get(&ctx.db, app, row.id).await.unwrap()
}
async fn prepared(
    server: &TestServer,
    ctx: &AppContext,
    owner: &Login,
) -> (Uuid, Uuid, Uuid, worker_auth::Worker, String) {
    let mut input = create_input(owner.org);
    input.android_package = "ai.mobileqa.demo".into();
    let app = apps::create(ctx, owner.user, input).await.unwrap();
    let bytes = fixture("execution");
    let upload = new_upload(server, owner, app.id, &bytes).await;
    transfer(server, owner, app.id, upload.id, bytes.clone())
        .await
        .assert_status_ok();
    let res = owner
        .write(server.post(&format!(
            "/api/apps/{}/build-uploads/{}/complete",
            app.id, upload.id
        )))
        .await;
    res.assert_status_ok();
    let build = res.json::<BuildResponse>();
    assert_eq!(build.validation.state, ValidationState::Validated);
    let profile = ExecutionProfile {
        execution_context: None,
        id: Uuid::new_v4(),
        name: "Synthetic worker".into(),
        driver: Driver::Fake,
        package: "ai.mobileqa.demo".into(),
        adapter: "demo_persistence_v1".into(),
        device_identity: Uuid::new_v4().to_string(),
        image: "synthetic".into(),
        model: None,
        qualified: true,
        qualification_reference: "synthetic-fixture-only".into(),
        max_apk_bytes: 262144000,
    };
    defs::register_profile(ctx, owner.user, app.id, profile.clone())
        .await
        .unwrap();
    let case = saved_definition(ctx, owner, app.id, definition()).await;
    let plan = TestDefinition::Plan(PlanDefinition {
        key: "release".into(),
        version: 1,
        title: "Release check".into(),
        suite_version_ids: vec![],
        cases: vec![CaseSelection {
            case_version_id: case.id,
            data_variant: "default".into(),
            required: true,
        }],
        profile_id: profile.id,
        budget: ExecutionBudget {
            duration_seconds: 1200,
            max_steps: 30,
            artifact_bytes: 16777216,
        },
        diagnostic_retries: 0,
        exclusions: vec![],
    });
    let plan = saved_definition(ctx, owner, app.id, plan).await;
    mobile_qa::services::test_library_mutations::apply(
        ctx,
        owner.user,
        app.id,
        mobile_qa::services::test_library_mutations::Mutation::Default(
            mobile_qa_contracts::test_library::SetDefaultPlanRequest {
                mutation_id: Uuid::new_v4(),
                expected_revision: 0,
                plan_version_id: plan.id,
            },
        ),
    )
    .await
    .unwrap();
    let worker = worker_auth::Worker {
        id: Uuid::new_v4(),
        app_id: app.id,
        profile_id: profile.id,
    };
    let token = loco_rs::hash::random_string(64);
    worker_auth::register(ctx, owner.user, app.id, worker.id, profile.id, &token)
        .await
        .unwrap();
    (app.id, build.id, plan.id, worker, token)
}
#[tokio::test]
async fn route_manifest_idempotency_and_worker_fencing() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let foreign = login(&server, &ctx).await;
        let (app, build, plan, w, token) = prepared(&server, &ctx, &owner).await;
        let idle = scheduler::claim(
            &ctx,
            &w,
            ClaimRequest {
                version: 1,
                claim_id: Uuid::new_v4(),
                profile_id: w.profile_id,
                model_capabilities: None,
            },
        )
        .await
        .unwrap();
        assert!(idle.lease.is_none());
        assert!(field::<bool>(
            &one(
                &ctx.db,
                "SELECT model_last_seen_at IS NOT NULL AS seen FROM execution_workers WHERE id=$1",
                vec![w.id.into()]
            )
            .await
            .unwrap(),
            "seen"
        )
        .unwrap());
        let input = CreateRunRequest {
            build_id: build,
            plan_version_id: plan,
            environment_revision: 1,
        };
        let url = format!("/api/apps/{app}/runs");
        let submit = || {
            owner
                .write(server.post(&url))
                .add_header("idempotency-key", "same-click")
                .json(&input)
        };
        let first = submit().await;
        first.assert_status(axum::http::StatusCode::CREATED);
        let run = first.json::<RunResponse>();
        let replay = submit().await;
        replay.assert_status_ok();
        assert_eq!(replay.json::<RunResponse>().id, run.id);
        error(
            &foreign
                .read(server.get(&format!("/api/runs/{}", run.id)))
                .await,
            404,
        );
        let mut changed = input.clone();
        changed.environment_revision = 2;
        error(
            &owner
                .write(server.post(&url))
                .add_header("idempotency-key", "same-click")
                .json(&changed)
                .await,
            409,
        );
        let frozen = run.manifest.clone();
        let case_id = frozen.cases[0].definition_id;
        let row = one(
            &ctx.db,
            "SELECT entry_id FROM test_library_versions WHERE definition_id=$1",
            vec![case_id.into()],
        )
        .await
        .unwrap();
        let entry = mobile_qa::services::test_library::entry(
            &ctx.db,
            owner.user,
            app,
            field(&row, "entry_id").unwrap(),
        )
        .await
        .unwrap();
        mobile_qa::services::test_library_mutations::apply(
            &ctx,
            owner.user,
            app,
            mobile_qa::services::test_library_mutations::Mutation::Archive(
                entry.id,
                mobile_qa_contracts::test_library::ArchiveLibraryEntryRequest {
                    mutation_id: Uuid::new_v4(),
                    expected_revision: entry.revision,
                    archived: true,
                },
            ),
        )
        .await
        .unwrap();
        assert!(!runs::preview(&ctx.db, app, build, Some(plan))
            .await
            .unwrap()
            .blockers
            .is_empty());
        assert_eq!(
            runs::detail(&ctx.db, run.id).await.unwrap().manifest,
            frozen
        );
        let cid = Uuid::new_v4();
        let response = server
            .post("/api/worker/claims")
            .add_header("authorization", format!("Bearer {token}"))
            .json(&ClaimRequest {
                version: 1,
                claim_id: cid,
                profile_id: w.profile_id,
                model_capabilities: None,
            })
            .await;
        response.assert_status_ok();
        let lease = response.json::<ClaimResponse>().lease.unwrap();
        assert_eq!(lease.manifest, frozen);
        let second = scheduler::claim(
            &ctx,
            &w,
            ClaimRequest {
                version: 1,
                claim_id: Uuid::new_v4(),
                profile_id: w.profile_id,
                model_capabilities: None,
            },
        )
        .await
        .unwrap();
        assert!(second.lease.is_none());
        let again = scheduler::claim(
            &ctx,
            &w,
            ClaimRequest {
                version: 1,
                claim_id: cid,
                profile_id: w.profile_id,
                model_capabilities: None,
            },
        )
        .await
        .unwrap()
        .lease
        .unwrap();
        assert_eq!(again.attempt_id, lease.attempt_id);
        assert_ne!(again.lease_token, lease.lease_token);
        assert!(scheduler::heartbeat(
            &ctx,
            &w,
            lease.attempt_id,
            lease.generation,
            &lease.lease_token
        )
        .await
        .is_err());
        let lease = again;
        let ev = ExecutionEvent {
            id: Uuid::new_v4(),
            sequence: 1,
            action_id: "create".into(),
            phase: "navigation".into(),
            message: "synthetic".into(),
        };
        let events = EventRequest {
            generation: lease.generation,
            events: vec![ev.clone()],
        };
        scheduler::events(
            &ctx,
            &w,
            lease.attempt_id,
            &lease.lease_token,
            events.clone(),
        )
        .await
        .unwrap();
        scheduler::events(&ctx, &w, lease.attempt_id, &lease.lease_token, events)
            .await
            .unwrap();
        let mut bad = ev;
        bad.message = "different".into();
        assert!(scheduler::events(
            &ctx,
            &w,
            lease.attempt_id,
            &lease.lease_token,
            EventRequest {
                generation: lease.generation,
                events: vec![bad]
            }
        )
        .await
        .is_err());
        let cancel = owner
            .write(server.post(&format!("/api/runs/{}/cancel", run.id)))
            .await;
        cancel.assert_status_ok();
        assert_eq!(
            cancel.json::<RunResponse>().state,
            JobState::CancelRequested
        );
        let completion = CompleteRequest {
            generation: lease.generation,
            execution_outcome: Outcome::Passed,
            reason: "sdk_completed_without_evidence".into(),
            usage: vec![],
        };
        let receipt =
            scheduler::complete(&ctx, &w, lease.attempt_id, &lease.lease_token, completion)
                .await
                .unwrap();
        assert_eq!(receipt.attempt.outcome, Some(Outcome::Canceled));
        exec(
            &ctx.db,
            "UPDATE execution_attempts SET expires_at=now()-interval '1 second' WHERE id=$1",
            vec![lease.attempt_id.into()],
        )
        .await
        .unwrap();
        scheduler::reconcile(&ctx).await.unwrap();
        let r = runs::detail(&ctx.db, run.id).await.unwrap();
        assert_eq!(r.state, JobState::RecoveryRequired);
        assert!(!rows(
            &ctx.db,
            "SELECT * FROM execution_reservations WHERE attempt_id=$1",
            vec![lease.attempt_id.into()]
        )
        .await
        .unwrap()
        .is_empty());
        assert!(scheduler::cleanup(
            &ctx,
            &w,
            lease.attempt_id,
            &lease.lease_token,
            CleanupRequest {
                generation: lease.generation,
                stopped: true,
                reset: CleanupState::VerifiedClean,
                evidence_reference: "late".into(),
                boot_id: "synthetic".into()
            }
        )
        .await
        .is_err());
    })
    .await;
}
#[tokio::test]
async fn real_database_competing_claims_and_immutable_definition_boundaries() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let (app, build, plan, w, _) = prepared(&server, &ctx, &owner).await;
        let d = defs::get(&ctx.db, app, plan).await.unwrap();
        let mut altered = d.definition;
        let TestDefinition::Plan(ref mut p) = altered else {
            panic!()
        };
        p.title = "Changed after saving".into();
        assert!(defs::import(
            &ctx,
            owner.user,
            DefinitionImport {
                app_id: app,
                definition: altered
            }
        )
        .await
        .is_err());
        for k in ["a", "b"] {
            runs::create(
                &ctx,
                owner.user,
                app,
                k,
                CreateRunRequest {
                    build_id: build,
                    plan_version_id: plan,
                    environment_revision: 1,
                },
            )
            .await
            .unwrap();
        }
        let (a, b) = tokio::join!(
            scheduler::claim(
                &ctx,
                &w,
                ClaimRequest {
                    version: 1,
                    claim_id: Uuid::new_v4(),
                    profile_id: w.profile_id,
                    model_capabilities: None,
                }
            ),
            scheduler::claim(
                &ctx,
                &w,
                ClaimRequest {
                    version: 1,
                    claim_id: Uuid::new_v4(),
                    profile_id: w.profile_id,
                    model_capabilities: None,
                }
            )
        );
        assert_eq!(
            usize::from(a.unwrap().lease.is_some()) + usize::from(b.unwrap().lease.is_some()),
            1
        );
        // Independent transactions see the same unreleased app reservation.
        let tx = ctx.db.begin().await.unwrap();
        let reservations = rows(
            &tx,
            "SELECT resource FROM execution_reservations WHERE resource=$1",
            vec![format!("app:{app}").into()],
        )
        .await
        .unwrap();
        assert_eq!(reservations.len(), 1);
        tx.rollback().await.unwrap();
    })
    .await;
}
#[test]
fn evidence_verifier_distinguishes_missing_task_from_missing_prerequisite() {
    let TestDefinition::Case(c) = definition() else {
        panic!()
    };
    let check = &c.checks[1];
    let ready=b"<hierarchy><node package=\"ai.mobileqa.demo\" resource-id=\"ai.mobileqa.demo:id/ready_marker\"/></hierarchy>";
    assert_eq!(
        verification::observe(ready, &c.package, check, "qa-one").unwrap(),
        "false"
    );
    assert!(
        verification::observe(&ready[..ready.len() - 12], &c.package, check, "qa-one").is_err()
    );
    assert!(
        verification::observe(b"<hierarchy/><hierarchy/>", &c.package, check, "qa-one").is_err()
    );
    assert!(verification::observe(b"<hierarchy/>", &c.package, check, "qa-one").is_err());
    assert!(
        verification::observe(b"<!DOCTYPE a><hierarchy/>", &c.package, check, "qa-one").is_err()
    );
    assert!(!verification::png_valid(b"not a screenshot"));
}
#[tokio::test]
async fn execution_route_inventory_has_actual_auth_and_errors() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, _ctx| async move {
        let spec = serde_json::to_value(mobile_qa_contracts::browser::openapi()).unwrap();
        for &(method, path, operation, status) in mobile_qa_contracts::execution_api::OPERATIONS {
            assert_eq!(spec["paths"][path][method]["operationId"], operation);
            assert!(spec["paths"][path][method]["responses"][status.to_string()].is_object());
            let concrete = path
                .replace("{app_id}", &Uuid::new_v4().to_string())
                .replace("{run_id}", &Uuid::new_v4().to_string())
                .replace("{artifact_id}", &Uuid::new_v4().to_string());
            let req = if method == "get" {
                server.get(&concrete)
            } else {
                server.post(&concrete)
            };
            error(&req.await, 401);
        }
        for suffix in ["heartbeat", "events", "artifacts", "complete", "cleanup"] {
            error(
                &server
                    .post(&format!("/api/worker/attempts/{}/{suffix}", Uuid::new_v4()))
                    .await,
                401,
            );
        }
        error(&server.post("/api/worker/claims").await, 401);
    })
    .await;
}
#[tokio::test]
async fn evidence_http_roundtrip_pass_failure_and_blocked() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App,_,_>(|server,ctx|async move{
 let owner=login(&server,&ctx).await;let(app,build,plan,w,token)=prepared(&server,&ctx,&owner).await;
 for (scenario,expected) in [("pass",Outcome::Passed),("fail",Outcome::Failed),("blocked",Outcome::Blocked)]{
 let(run,_)=runs::create(&ctx,owner.user,app,scenario,CreateRunRequest{build_id:build,plan_version_id:plan,environment_revision:1}).await.unwrap();
 let lease=scheduler::claim(&ctx,&w,ClaimRequest{version:1,claim_id:Uuid::new_v4(),profile_id:w.profile_id,model_capabilities:None}).await.unwrap().lease.unwrap();
 let prefix=format!("/api/worker/attempts/{}",lease.attempt_id);let mut artifact_id=Uuid::nil();
 for cp in ["created","reopened"]{
 let task=format!("qa-{}",lease.attempt_id);let marker=if scenario=="blocked"{"prerequisite_unavailable"}else{"ready_marker"};
 let task_node=if scenario=="fail"&&cp=="reopened"{String::new()}else{format!("<node package=\"ai.mobileqa.demo\" resource-id=\"ai.mobileqa.demo:id/task_row\" text=\"{task}\"/>")};
 let xml=format!("<hierarchy><node package=\"ai.mobileqa.demo\" resource-id=\"ai.mobileqa.demo:id/{marker}\"/>{task_node}</hierarchy>").into_bytes();
 for (suffix,mime,data) in [("xml","application/xml",xml),("png","image/png",include_bytes!("fixtures/execution/synthetic.png").to_vec())]{
 let body=ArtifactRequest{generation:lease.generation,checkpoint_id:cp.into(),name:format!("{cp}.{suffix}"),mime:mime.into(),byte_size:data.len() as u32,sha256:hash(&data)};
 let reserve=server.post(&format!("{prefix}/artifacts")).add_header("authorization",format!("Bearer {token}")).add_header("x-lease-token",&lease.lease_token).json(&body).await;
 reserve.assert_status(axum::http::StatusCode::CREATED);let a=reserve.json::<ArtifactReceipt>().artifact;artifact_id=a.id;
 let content_url=format!("{prefix}/artifacts/{}/content",a.id);
 for _ in 0..2 {let upload=server.put(&content_url).add_header("authorization",format!("Bearer {token}")).add_header("x-lease-token",&lease.lease_token).add_header("x-lease-generation",lease.generation.to_string()).bytes(axum::body::Bytes::from(data.clone())).await;upload.assert_status_ok();}
 }
 }
 let complete=CompleteRequest{generation:lease.generation,execution_outcome:if scenario=="blocked"{Outcome::Blocked}else{Outcome::Passed},reason:"synthetic_completed".into(),usage:vec![]};
 let result=server.post(&format!("{prefix}/complete")).add_header("authorization",format!("Bearer {token}")).add_header("x-lease-token",&lease.lease_token).json(&complete).await;result.assert_status_ok();assert_eq!(result.json::<AttemptReceipt>().attempt.outcome,Some(expected));
 scheduler::complete(&ctx,&w,lease.attempt_id,&lease.lease_token,complete).await.unwrap();
 let cleanup=CleanupRequest{generation:lease.generation,stopped:true,reset:CleanupState::VerifiedClean,evidence_reference:"synthetic-reset".into(),boot_id:"synthetic".into()};
 scheduler::cleanup(&ctx,&w,lease.attempt_id,&lease.lease_token,cleanup.clone()).await.unwrap();scheduler::cleanup(&ctx,&w,lease.attempt_id,&lease.lease_token,cleanup).await.unwrap();
 let result=owner.read(server.get(&format!("/api/runs/{}",run.id))).await;result.assert_status_ok();assert_eq!(result.json::<RunResponse>().state,JobState::Finished);
 owner.read(server.get(&format!("/api/runs/{}/artifacts/{artifact_id}/content",run.id))).await.assert_status_ok();
 assert_eq!(runs::detail(&ctx.db,run.id).await.unwrap().attempts[0].outcome,Some(expected));
 }
 }).await;
}

#[tokio::test]
async fn long_poll_wakes_on_committed_run_and_times_out_without_reserving() {
    use std::time::Duration;
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let (app, build, plan, worker, token) = prepared(&server, &ctx, &owner).await;
        let request = ClaimRequest {
            version: 1,
            claim_id: Uuid::new_v4(),
            profile_id: worker.profile_id,
            model_capabilities: None,
        };
        let start = tokio::time::Instant::now();
        let idle =
            scheduler::claim_wait(&ctx, &worker, request.clone(), Duration::from_millis(100))
                .await
                .unwrap();
        assert!(idle.lease.is_none());
        assert_eq!(idle.poll_after_seconds, 0);
        assert!(start.elapsed() >= Duration::from_millis(100));
        assert!(rows(
            &ctx.db,
            "SELECT resource FROM execution_reservations WHERE resource=$1",
            vec![format!("app:{app}").into()]
        )
        .await
        .unwrap()
        .is_empty());
        // Start the actual HTTP request first. Submission must wake it before the
        // five-second fallback, and only after the run transaction commits.
        let claim = async {
            server
                .post("/api/worker/claims")
                .add_header("authorization", format!("Bearer {token}"))
                .json(&request)
                .await
        };
        let enqueue = async {
            tokio::time::sleep(Duration::from_millis(150)).await;
            runs::create(
                &ctx,
                owner.user,
                app,
                "wake-test",
                CreateRunRequest {
                    build_id: build,
                    plan_version_id: plan,
                    environment_revision: 1,
                },
            )
            .await
            .unwrap()
            .0
        };
        let start = tokio::time::Instant::now();
        let (response, run) = tokio::time::timeout(Duration::from_secs(3), async {
            tokio::join!(claim, enqueue)
        })
        .await
        .expect("queue commit must wake waiting HTTP claim");
        response.assert_status_ok();
        let lease = response.json::<ClaimResponse>().lease.unwrap();
        assert_eq!(lease.run_id, run.id);
        assert!(start.elapsed() >= Duration::from_millis(150));
        assert_eq!(lease.manifest.build_id, build);
    })
    .await;
}

#[tokio::test]
async fn long_poll_rechecks_revocation_after_waking() {
    use std::time::Duration;
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let (_, _, _, worker, _) = prepared(&server, &ctx, &owner).await;
        let waiting = scheduler::claim_wait(
            &ctx,
            &worker,
            ClaimRequest {
                version: 1,
                claim_id: Uuid::new_v4(),
                profile_id: worker.profile_id,
                model_capabilities: None,
            },
            Duration::from_secs(2),
        );
        let revoke = async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            exec(
                &ctx.db,
                "UPDATE execution_workers SET revoked=true WHERE id=$1",
                vec![worker.id.into()],
            )
            .await
            .unwrap();
            mobile_qa::services::execution_wakeup::notify(&ctx);
        };
        let (result, ()) = tokio::join!(waiting, revoke);
        assert_eq!(result.unwrap_err().code, "unauthenticated");
    })
    .await;
}

#[tokio::test]
async fn clean_start_route_is_fenced_and_recovery_keeps_original_evidence() {
    use mobile_qa_contracts::execution_lifecycle::*;
    let _guard = DATABASE_BOOT.lock().await;
    request::<App,_,_>(|server,ctx|async move {
        let owner=login(&server,&ctx).await;
        let(app,build,_,_,_)=prepared(&server,&ctx,&owner).await;
        // The APK is a synthetic fixture; this exercises generic admission, not real-app qualification.
        let mut p=ExecutionProfile {execution_context:None,id:Uuid::new_v4(),name:"Direct fixture".into(),driver:Driver::Direct,package:"ai.mobileqa.demo".into(),adapter:"android_direct_v1".into(),device_identity:Uuid::new_v4().to_string(),image:"system-images;android-35;google_apis;x86_64".into(),model:None,qualified:true,qualification_reference:"synthetic local-state qualification".into(),max_apk_bytes:104857600};
        let start=ExpectedCheck {id:"empty".into(),checkpoint_id:"preflight".into(),description:"Input empty".into(),method:CheckMethod::UiPropertyEqualsV1,resource_id:"ai.mobileqa.demo:id/task_input".into(),text_filter:String::new(),property:UiProperty::Text,expected:String::new(),ready_resource_id:"ai.mobileqa.demo:id/task_input".into(),prerequisite_check_ids:vec![],required:true,observation_seconds:1};
        p.execution_context=Some(ExecutionContextV1 {schema_version:1,adapter_revision:p.adapter.clone(),verifier_revision:"ui_v1".into(),worker_runtime_revision:"direct_v1".into(),reset_policy_hash:"a".repeat(64),qualified_profile_id:p.id,package:p.package.clone(),launch_component:"ai.mobileqa.demo/.MainActivity".into(),image:p.image.clone(),abi:"x86_64".into(),width:1080,height:1920,density:420,locale:"en-US".into(),timezone:"Etc/UTC".into(),state_scope:"local_only".into(),qualification_reference:p.qualification_reference.clone(),starting_checks:vec![start],stages:StageBudgets{boot_seconds:60,install_seconds:60,start_seconds:30,cleanup_seconds:60}});
        defs::register_profile(&ctx,owner.user,app,p.clone()).await.unwrap();
        let TestDefinition::Case(mut c)=definition() else {panic!()};
        c.key="direct-clean".into();c.adapter=p.adapter.clone();
        for a in &mut c.actions {if a.kind==ActionKind::Navigate {a.kind=ActionKind::Checkpoint;a.instruction.clear();}}
        let case=saved_definition(&ctx,&owner,app,TestDefinition::Case(c)).await;
        let plan=saved_definition(&ctx,&owner,app,TestDefinition::Plan(PlanDefinition{key:"direct-clean-plan".into(),version:1,title:"Clean replay".into(),suite_version_ids:vec![],cases:vec![CaseSelection{case_version_id:case.id,data_variant:"default".into(),required:true}],profile_id:p.id,budget:ExecutionBudget{duration_seconds:1800,max_steps:80,artifact_bytes:16777216},diagnostic_retries:0,exclusions:vec![]})).await;
        let w=worker_auth::Worker{id:Uuid::new_v4(),app_id:app,profile_id:p.id};let token=loco_rs::hash::random_string(64);
        worker_auth::register(&ctx,owner.user,app,w.id,p.id,&token).await.unwrap();
        let(run,_)=runs::create(&ctx,owner.user,app,"clean",CreateRunRequest{build_id:build,plan_version_id:plan.id,environment_revision:1}).await.unwrap();
        for version in [1,2] {assert!(scheduler::claim(&ctx,&w,ClaimRequest{version,claim_id:Uuid::new_v4(),profile_id:p.id,model_capabilities:None}).await.unwrap().lease.is_none());}
        let lease=scheduler::claim(&ctx,&w,ClaimRequest{version:3,claim_id:Uuid::new_v4(),profile_id:p.id,model_capabilities:None}).await.unwrap().lease.unwrap();
        let prefix=format!("/api/worker/attempts/{}",lease.attempt_id);
        let event=EventRequest{generation:lease.generation,events:vec![ExecutionEvent{id:Uuid::new_v4(),sequence:1,action_id:"create".into(),phase:"started".into(),message:"Starting".into()}]};
        error(&server.post(&format!("{prefix}/events")).add_header("authorization",format!("Bearer {token}")).add_header("x-lease-token",&lease.lease_token).json(&event).await,409);
        let mut ids=vec![];
        for(suffix,mime,data)in[("xml","application/xml",b"<hierarchy><node package=\"ai.mobileqa.demo\" resource-id=\"ai.mobileqa.demo:id/task_input\" text=\"\"/></hierarchy>".to_vec()),("png","image/png",include_bytes!("fixtures/execution/synthetic.png").to_vec())] {
            let body=ArtifactRequest{generation:lease.generation,checkpoint_id:"preflight".into(),name:format!("preflight.{suffix}"),mime:mime.into(),byte_size:data.len() as u32,sha256:hash(&data)};
            let response=server.post(&format!("{prefix}/artifacts")).add_header("authorization",format!("Bearer {token}")).add_header("x-lease-token",&lease.lease_token).json(&body).await;response.assert_status(axum::http::StatusCode::CREATED);
            let a=response.json::<ArtifactReceipt>().artifact;ids.push(a.id);
            server.put(&format!("{prefix}/artifacts/{}/content",a.id)).add_header("authorization",format!("Bearer {token}")).add_header("x-lease-token",&lease.lease_token).add_header("x-lease-generation",lease.generation.to_string()).bytes(axum::body::Bytes::from(data)).await.assert_status_ok();
        }
        let now=chrono::Utc::now();
        let request=PreflightRequest{generation:lease.generation,receipt:PreflightReceipt{attempt_id:lease.attempt_id,instance_nonce:Uuid::new_v4(),context:p.execution_context.clone().unwrap(),build_sha256:run.manifest.build_sha256.clone(),started_at:now,ready_at:now,duration_ms:0,artifact_ids:ids}};
        let url=format!("{prefix}/preflight");
        error(&server.post(&url).json(&request).await,401);
        for _ in 0..2 {let response=server.post(&url).add_header("authorization",format!("Bearer {token}")).add_header("x-lease-token",&lease.lease_token).json(&request).await;response.assert_status_ok();assert!(response.json::<PreflightAcknowledgement>().accepted);}
        let mut changed=request.clone();changed.receipt.instance_nonce=Uuid::new_v4();
        error(&server.post(&url).add_header("authorization",format!("Bearer {token}")).add_header("x-lease-token",&lease.lease_token).json(&changed).await,409);
        changed=request.clone();changed.generation+=1;
        error(&server.post(&url).add_header("authorization",format!("Bearer {token}")).add_header("x-lease-token",&lease.lease_token).json(&changed).await,409);
        scheduler::events(&ctx,&w,lease.attempt_id,&lease.lease_token,event).await.unwrap();
        let completion=CompleteRequest{generation:lease.generation,execution_outcome:Outcome::Passed,reason:"fixture has no action proof".into(),usage:vec![]};
        assert_ne!(scheduler::complete(&ctx,&w,lease.attempt_id,&lease.lease_token,completion).await.unwrap().attempt.outcome,Some(Outcome::Passed));
        let cleanup=CleanupRequest{generation:lease.generation,stopped:false,reset:CleanupState::Quarantined,evidence_reference:"stop uncertain".into(),boot_id:"fixture".into()};
        scheduler::cleanup(&ctx,&w,lease.attempt_id,&lease.lease_token,cleanup.clone()).await.unwrap();
        scheduler::recover(&ctx,owner.user,app,lease.attempt_id,"owned instance disposed").await.unwrap();
        let a=runs::attempt(&ctx.db,lease.attempt_id).await.unwrap();
        assert_eq!(a.original_cleanup,Some(cleanup));assert_eq!(a.recovery_events.len(),1);assert_eq!(a.preflight,Some(request.receipt));assert_ne!(a.outcome,Some(Outcome::Passed));
    }).await;
}

#[tokio::test]
async fn saved_case_routes_pin_inputs_retry_once_and_use_protocol_four() {
    use mobile_qa_contracts::regression::*;
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let foreign = login(&server, &ctx).await;
        let (app, build, plan, worker, _) = prepared(&server, &ctx, &owner).await;
        let preview = runs::preview(&ctx.db, app, build, Some(plan))
            .await
            .unwrap();
        let m = preview.manifest.unwrap();
        let body = CaseRunRequest {
            case_version_id: m.cases[0].definition_id,
            build_id: build,
            profile_id: worker.profile_id,
            environment_revision: 1,
            baseline_run_id: None,
        };
        let path = format!("/api/apps/{app}/case-runs");
        error(&server.post(&path).json(&body).await, 401);
        assert!(!foreign
            .write(server.post(&path))
            .add_header("idempotency-key", "case-run")
            .json(&body)
            .await
            .status_code()
            .is_success());
        owner
            .write(server.post(&format!("{path}/preview")))
            .json(&body)
            .await
            .assert_status_ok();
        let response = owner
            .write(server.post(&path))
            .add_header("idempotency-key", "case-run")
            .json(&body)
            .await;
        response.assert_status(axum::http::StatusCode::CREATED);
        let first = response.json::<RunResponse>();
        assert_eq!(
            first.manifest.source,
            Some(RunSource::SavedCaseV1 {
                case_version_id: body.case_version_id
            })
        );
        assert!(first.manifest.plan_version_id.is_none());
        let retry = owner
            .write(server.post(&path))
            .add_header("idempotency-key", "case-run")
            .json(&body)
            .await;
        retry.assert_status_ok();
        assert_eq!(retry.json::<RunResponse>().id, first.id);
        let mut changed = body.clone();
        changed.environment_revision = 2;
        error(
            &owner
                .write(server.post(&path))
                .add_header("idempotency-key", "case-run")
                .json(&changed)
                .await,
            409,
        );
        error(
            &owner
                .write(server.post(&path))
                .add_header("idempotency-key", "stale")
                .json(&changed)
                .await,
            409,
        );
        for version in [1, 2, 3] {
            assert!(scheduler::claim(
                &ctx,
                &worker,
                ClaimRequest {
                    version,
                    claim_id: Uuid::new_v4(),
                    profile_id: worker.profile_id,
                    model_capabilities: None,
                }
            )
            .await
            .unwrap()
            .lease
            .is_none());
        }
        let claim = ClaimRequest {
            version: 4,
            claim_id: Uuid::new_v4(),
            profile_id: worker.profile_id,
            model_capabilities: None,
        };
        assert_eq!(
            scheduler::claim(&ctx, &worker, claim.clone())
                .await
                .unwrap()
                .lease
                .unwrap()
                .run_id,
            first.id
        );
        error(
            &owner
                .write(server.post(&path))
                .add_header("idempotency-key", "active-baseline")
                .json(&CaseRunRequest {
                    baseline_run_id: Some(first.id),
                    ..body.clone()
                })
                .await,
            409,
        );
        let history = owner
            .read(server.get(&format!("/api/apps/{app}/run-history")))
            .await;
        history.assert_status_ok();
        let h = history.json::<RunHistory>();
        assert_eq!(h.items.len(), 1);
        assert_eq!(h.items[0].run.as_ref().unwrap().id, first.id);
        assert!(!foreign
            .read(server.get(&format!("/api/apps/{app}/run-history")))
            .await
            .status_code()
            .is_success());
        // A read does not finalize or mutate an unfinished comparison.
        assert!(runs::detail(&ctx.db, first.id)
            .await
            .unwrap()
            .comparison
            .is_none());
        runs::cancel(&ctx, owner.user, first.id).await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn saved_suite_route_queues_each_numbered_case_in_one_immutable_run() {
    use mobile_qa_contracts::regression::*;
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let (app, build, plan, worker, _) = prepared(&server, &ctx, &owner).await;
        let TestDefinition::Plan(plan_definition) =
            defs::get(&ctx.db, app, plan).await.unwrap().definition
        else {
            panic!("fixture plan expected")
        };
        let first_case = plan_definition.cases[0].case_version_id;
        let TestDefinition::Case(mut second_definition) = definition() else {
            panic!("fixture case expected")
        };
        second_definition.key = "second-saved-step".into();
        second_definition.title = "Second saved step".into();
        let second_case =
            saved_definition(&ctx, &owner, app, TestDefinition::Case(second_definition)).await;
        let suite = saved_definition(
            &ctx,
            &owner,
            app,
            TestDefinition::Suite(SuiteDefinition {
                key: "ordered-suite".into(),
                version: 1,
                title: "Ordered suite".into(),
                cases: vec![
                    CaseSelection {
                        case_version_id: first_case,
                        data_variant: "default".into(),
                        required: true,
                    },
                    CaseSelection {
                        case_version_id: second_case.id,
                        data_variant: "default".into(),
                        required: true,
                    },
                ],
            }),
        )
        .await;
        let body = SuiteRunRequest {
            suite_version_id: suite.id,
            build_id: build,
            profile_id: worker.profile_id,
            environment_revision: 1,
        };
        let path = format!("/api/apps/{app}/suite-runs");
        let preview = owner
            .write(server.post(&format!("{path}/preview")))
            .json(&body)
            .await;
        preview.assert_status_ok();
        assert!(preview.json::<SuiteRunPreview>().blockers.is_empty());
        let response = owner
            .write(server.post(&path))
            .add_header("idempotency-key", "suite-run")
            .json(&body)
            .await;
        response.assert_status(axum::http::StatusCode::CREATED);
        let run = response.json::<RunResponse>();
        assert_eq!(
            run.manifest.source,
            Some(RunSource::SavedSuiteV1 {
                suite_version_id: suite.id
            })
        );
        assert!(run.manifest.plan_version_id.is_none());
        assert_eq!(run.manifest.cases.len(), 2);
        assert_eq!(run.attempts.len(), 2);
        assert_eq!(run.attempts[0].case_version_id, first_case);
        assert_eq!(run.attempts[1].case_version_id, second_case.id);
        let retry = owner
            .write(server.post(&path))
            .add_header("idempotency-key", "suite-run")
            .json(&body)
            .await;
        retry.assert_status_ok();
        assert_eq!(retry.json::<RunResponse>().id, run.id);
    })
    .await;
}

#[test]
fn assertion_absence_requires_a_ready_unambiguous_screen() {
    let TestDefinition::Case(c) = definition() else {
        panic!()
    };
    let mut check = c.checks[0].clone();
    check.method = CheckMethod::UiPropertyEqualsV1;
    let xml=br#"<hierarchy><node package="ai.mobileqa.demo" resource-id="ai.mobileqa.demo:id/ready_marker" text="Ready"/></hierarchy>"#;
    assert_eq!(
        verification::observe(xml, &c.package, &check, "demo"),
        Err("target_absent_on_ready_screen")
    );
    check.ready_resource_id = "ai.mobileqa.demo:id/other_screen".into();
    assert_eq!(
        verification::observe(xml, &c.package, &check, "demo"),
        Err("screen_not_ready")
    );
}

#[tokio::test]
async fn comparison_finalization_is_durable_idempotent_and_baseline_is_pinned() {
    use mobile_qa::services::{case_runs, run_comparisons};
    use mobile_qa_contracts::regression::*;
    let _guard = DATABASE_BOOT.lock().await;
    request::<App,_,_>(|server,ctx|async move {
        let owner=login(&server,&ctx).await;
        let(app,build,_,_,_)=prepared(&server,&ctx,&owner).await;
        let TestDefinition::Case(mut case)=definition() else {panic!()};
        case.key="comparison-direct".into();case.adapter="android_direct_v1".into();
        for action in &mut case.actions {if action.kind==ActionKind::Navigate {action.kind=ActionKind::Checkpoint;action.instruction.clear();}}
        case.checks.truncate(1);case.checks[0].required=true;
        let case=saved_definition(&ctx,&owner,app,TestDefinition::Case(case)).await;
        let fixture:RunResponse=serde_json::from_str(include_str!("fixtures/execution/comparison.json")).unwrap();
        let mut profile=fixture.manifest.profile;
        profile.id=Uuid::new_v4();
        // The serialized fixture is historical; a newly registered direct profile uses null.
        profile.model=None;
        let context=profile.execution_context.as_mut().unwrap();
        context.qualified_profile_id=profile.id;
        let starting_check=&mut context.starting_checks[0];
        starting_check.id="ready".into();starting_check.checkpoint_id="preflight".into();
        starting_check.text_filter.clear();starting_check.expected="true".into();
        starting_check.prerequisite_check_ids.clear();starting_check.required=true;
        defs::register_profile(&ctx,owner.user,app,profile.clone()).await.unwrap();
        let input=CaseRunRequest{case_version_id:case.id,build_id:build,profile_id:profile.id,environment_revision:1,baseline_run_id:None};
        let (base,_)=case_runs::create(&ctx,owner.user,app,"baseline",input.clone()).await.unwrap();
        // Seed retained synthetic facts to isolate persistence/finalization from worker transport.
        // Device/check publication and fencing are exercised by the route tests above.
        async fn finish(ctx:&AppContext,run:&RunResponse,failed:bool) {
            let mut fixture:RunResponse=serde_json::from_str(include_str!("fixtures/execution/comparison.json")).unwrap();
            fixture.manifest=run.manifest.clone();
            if failed {fixture.manifest.build_sha256="f".repeat(64);}
            let attempt=&mut fixture.attempts[0];attempt.id=run.attempts[0].id;attempt.case_version_id=run.manifest.cases[0].definition_id;
            let receipt=attempt.preflight.as_mut().unwrap();receipt.attempt_id=attempt.id;receipt.build_sha256=fixture.manifest.build_sha256.clone();receipt.context=fixture.manifest.profile.execution_context.clone().unwrap();
            let artifact=Uuid::new_v4();attempt.checks[0].artifact_ids=vec![artifact];
            if failed {attempt.checks[0].outcome=Outcome::Failed;attempt.checks[0].observed=Some("".into());}
            exec(&ctx.db,"UPDATE execution_runs SET manifest=$2 WHERE id=$1",vec![run.id.into(),json(&fixture.manifest).unwrap().into()]).await.unwrap();
            exec(&ctx.db,"UPDATE execution_attempts SET state='finished',outcome=$2,cleanup='verified_clean',checks=$3,cleanup_receipt=$4 WHERE id=$1",vec![attempt.id.into(),(if failed {"failed"}else{"passed"}).into(),json(&attempt.checks).unwrap().into(),json(&attempt.original_cleanup).unwrap().into()]).await.unwrap();
            exec(&ctx.db,"INSERT INTO execution_preflight_receipts(attempt_id,generation,digest,payload) VALUES($1,1,'fixture',$2)",vec![attempt.id.into(),json(receipt).unwrap().into()]).await.unwrap();
            exec(&ctx.db,"INSERT INTO execution_artifacts(id,attempt_id,checkpoint_id,name,mime,byte_size,sha256,state,storage_key,storage_backend) VALUES($1,$2,'created','created.xml','application/xml',1,'fixture','sealed',$3,'local')",vec![artifact.into(),attempt.id.into(),artifact.to_string().into()]).await.unwrap();
        }
        finish(&ctx,&base,false).await;
        // More than one page of newer, completed but incompatible runs must not hide the
        // latest eligible baseline. This guards the keyset scan in case-run previews.
        let noise_namespace = Uuid::new_v4().to_string();
        exec(
            &ctx.db,
            "INSERT INTO execution_runs(id,app_id,creator_id,build_id,plan_id,idempotency_key,fingerprint,manifest,cancel_requested,created_at)
             SELECT md5($2 || ':run:' || generated.position::text)::uuid,
                 app_id,creator_id,build_id,plan_id,$2 || ':key:' || generated.position::text,
                 fingerprint,jsonb_set(manifest,'{environment_revision}',to_jsonb(2::integer)),
                 cancel_requested,created_at + generated.position * interval '1 second'
             FROM execution_runs
             CROSS JOIN generate_series(1,101) AS generated(position)
             WHERE id=$1",
            vec![base.id.into(), noise_namespace.clone().into()],
        )
        .await
        .unwrap();
        exec(
            &ctx.db,
            "INSERT INTO execution_attempts(id,run_id,case_index,number,generation,state,outcome,cleanup,reason,checks,usage,completion_hash,cleanup_hash,cleanup_receipt)
             SELECT md5($2 || ':attempt:' || generated.position::text)::uuid,
                 md5($2 || ':run:' || generated.position::text)::uuid,
                 case_index,number,generation,state,outcome,cleanup,reason,checks,usage,
                 completion_hash,cleanup_hash,cleanup_receipt
             FROM execution_attempts
             CROSS JOIN generate_series(1,101) AS generated(position)
             WHERE run_id=$1",
            vec![base.id.into(), noise_namespace.into()],
        )
        .await
        .unwrap();
        let preview=case_runs::preview(&ctx,owner.user,app,input.clone()).await.unwrap();
        assert_eq!(preview.suggested_baseline_id,Some(base.id));
        let (current,_)=case_runs::create(&ctx,owner.user,app,"current",CaseRunRequest{baseline_run_id:Some(base.id),..input}).await.unwrap();
        finish(&ctx,&current,true).await;
        assert!(runs::detail(&ctx.db,current.id).await.unwrap().comparison.is_none());
        run_comparisons::finalize_pending(&ctx).await.unwrap();
        let snapshot=runs::detail(&ctx.db,current.id).await.unwrap();
        assert_eq!(snapshot.comparison.as_ref().unwrap().cases[0].kind,ComparisonKind::Regression);
        assert_eq!(snapshot.baseline_run_id,Some(base.id));
        run_comparisons::finalize_pending(&ctx).await.unwrap();
        let reloaded=owner.read(server.get(&format!("/api/runs/{}",current.id))).await.json::<RunResponse>();
        assert_eq!(reloaded.comparison,snapshot.comparison);
        assert_eq!(reloaded.baseline_run_id,snapshot.baseline_run_id);
    }).await;
}
