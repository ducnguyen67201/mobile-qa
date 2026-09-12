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
async fn approved(
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
    for purpose in [ApprovalPurpose::Business, ApprovalPurpose::Executability] {
        defs::grant(ctx, owner.user, app, owner.user, purpose)
            .await
            .unwrap();
        defs::approve(ctx, owner.user, app, row.id, &row.content_hash, purpose)
            .await
            .unwrap();
    }
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
        id: Uuid::new_v4(),
        name: "Synthetic worker".into(),
        driver: Driver::Fake,
        package: "ai.mobileqa.demo".into(),
        adapter: "demo_persistence_v1".into(),
        device_identity: Uuid::new_v4().to_string(),
        image: "synthetic".into(),
        model: "none".into(),
        qualified: true,
        qualification_reference: "synthetic-fixture-only".into(),
        max_apk_bytes: 262144000,
    };
    defs::register_profile(ctx, owner.user, app.id, profile.clone())
        .await
        .unwrap();
    let case = approved(ctx, owner, app.id, definition()).await;
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
    let plan = approved(ctx, owner, app.id, plan).await;
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
        let cid = Uuid::new_v4();
        let response = server
            .post("/api/worker/claims")
            .add_header("authorization", format!("Bearer {token}"))
            .json(&ClaimRequest {
                version: 1,
                claim_id: cid,
                profile_id: w.profile_id,
            })
            .await;
        response.assert_status_ok();
        let lease = response.json::<ClaimResponse>().lease.unwrap();
        let second = scheduler::claim(
            &ctx,
            &w,
            ClaimRequest {
                version: 1,
                claim_id: Uuid::new_v4(),
                profile_id: w.profile_id,
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
async fn real_database_competing_claims_and_approval_boundaries() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let (app, build, plan, w, _) = prepared(&server, &ctx, &owner).await;
        let d = defs::get(&ctx.db, app, plan).await.unwrap();
        assert!(defs::approve(
            &ctx,
            owner.user,
            app,
            plan,
            "stale",
            ApprovalPurpose::Business
        )
        .await
        .is_err());
        let mut altered = d.definition;
        let TestDefinition::Plan(ref mut p) = altered else {
            panic!()
        };
        p.title = "Changed after approval".into();
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
                    profile_id: w.profile_id
                }
            ),
            scheduler::claim(
                &ctx,
                &w,
                ClaimRequest {
                    version: 1,
                    claim_id: Uuid::new_v4(),
                    profile_id: w.profile_id
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
 let lease=scheduler::claim(&ctx,&w,ClaimRequest{version:1,claim_id:Uuid::new_v4(),profile_id:w.profile_id}).await.unwrap().lease.unwrap();
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
