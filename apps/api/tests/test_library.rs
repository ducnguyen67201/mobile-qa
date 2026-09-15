//! Real PostgreSQL and authenticated HTTP lifecycle agreement. Fixtures provision
//! app/profile/grants; every customer definition mutation goes through the router.
mod support;
use axum::http::StatusCode;
use mobile_qa::services::{execution_store::*, test_definitions as defs, test_library as library};
use mobile_qa_contracts::{execution::*, test_library::*};
use support::*;

async fn setup(server: &TestServer, ctx: &AppContext) -> (Login, Uuid, Uuid) {
    let owner = login(server, ctx).await;
    let mut input = create_input(owner.org);
    input.android_package = "ai.mobileqa.demo".into();
    let app = apps::create(ctx, owner.user, input).await.unwrap().id;
    let profile = ExecutionProfile {
        execution_context: None,
        id: Uuid::new_v4(),
        name: "Synthetic".into(),
        driver: Driver::Fake,
        package: "ai.mobileqa.demo".into(),
        adapter: "demo_persistence_v1".into(),
        device_identity: Uuid::new_v4().to_string(),
        image: "synthetic".into(),
        model: "none".into(),
        qualified: true,
        qualification_reference: "synthetic".into(),
        max_apk_bytes: 262144000,
    };
    defs::register_profile(ctx, owner.user, app, profile.clone())
        .await
        .unwrap();
    (owner, app, profile.id)
}
fn base(app: Uuid) -> String {
    format!("/api/apps/{app}/test-library")
}
fn failure(response: &TestResponse, status: u16) -> ApiError {
    assert_eq!(
        response.status_code().as_u16(),
        status,
        "{}",
        response.text()
    );
    let e = response.json::<ApiError>();
    assert_eq!(e.request_id.to_string(), response.header("x-request-id"));
    e
}
async fn create(
    server: &TestServer,
    owner: &Login,
    app: Uuid,
    kind: DefinitionKind,
    profile: Option<Uuid>,
) -> LibraryDraftResponse {
    let r = owner
        .write(server.post(&base(app)))
        .json(&CreateLibraryEntryRequest {
            mutation_id: Uuid::new_v4(),
            entry_id: Uuid::new_v4(),
            kind,
            key: Uuid::new_v4().to_string(),
            template_profile_id: profile,
        })
        .await;
    r.assert_status(StatusCode::CREATED);
    r.json()
}
async fn submit(
    server: &TestServer,
    owner: &Login,
    app: Uuid,
    draft: &LibraryDraftResponse,
) -> LibraryVersionResponse {
    let r = owner
        .write(server.post(&format!("{}/{}/submit", base(app), draft.entry.id)))
        .json(&SubmitLibraryDraftRequest {
            mutation_id: Uuid::new_v4(),
            expected_revision: draft.entry.revision,
        })
        .await;
    r.assert_status(StatusCode::CREATED);
    r.json()
}
async fn approve(
    server: &TestServer,
    ctx: &AppContext,
    owner: &Login,
    app: Uuid,
    mut candidate: LibraryVersionResponse,
) -> LibraryVersionResponse {
    for purpose in [ApprovalPurpose::Business, ApprovalPurpose::Executability] {
        defs::grant(ctx, owner.user, app, owner.user, purpose)
            .await
            .unwrap();
        let input = ReviewLibraryVersionRequest {
            mutation_id: Uuid::new_v4(),
            expected_revision: candidate.entry.revision,
            content_hash: candidate.version.content_hash.clone(),
            purpose,
            decision: LibraryReviewDecision::Approve,
            reason: None,
        };
        let path = format!(
            "{}/{}/versions/{}/review",
            base(app),
            candidate.entry.id,
            candidate.version.id
        );
        let r = owner.write(server.post(&path)).json(&input).await;
        r.assert_status_ok();
        candidate = r.json();
        let replay = owner.write(server.post(&path)).json(&input).await;
        replay.assert_status_ok();
        assert_eq!(replay.json::<LibraryVersionResponse>(), candidate);
    }
    assert_eq!(candidate.review_state, LibraryReviewState::Approved);
    candidate
}
#[tokio::test]
async fn incomplete_save_replay_conflict_and_atomic_publish() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let (owner, app, _) = setup(&server, &ctx).await;
        let mut draft = create(&server, &owner, app, DefinitionKind::Case, None).await;
        assert!(!draft.issues.is_empty());
        let path = format!("{}/{}/draft", base(app), draft.entry.id);
        let input = SaveLibraryDraftRequest {
            mutation_id: Uuid::new_v4(),
            expected_revision: draft.entry.revision,
            definition: draft.definition.clone(),
        };
        let saved = owner.write(server.put(&path)).json(&input).await;
        saved.assert_status_ok();
        draft = saved.json();
        let replay = owner.write(server.put(&path)).json(&input).await;
        replay.assert_status_ok();
        assert_eq!(replay.json::<LibraryDraftResponse>(), draft);
        let mut stale_input = input.clone();
        stale_input.mutation_id = Uuid::new_v4();
        let e = failure(
            &owner.write(server.put(&path)).json(&stale_input).await,
            409,
        );
        assert!(matches!(
            serde_json::from_value::<LibraryErrorDetails>(e.details.unwrap()).unwrap(),
            LibraryErrorDetails::StaleRevision { .. }
        ));
        stale_input.mutation_id = input.mutation_id;
        stale_input.expected_revision = draft.entry.revision;
        let e = failure(
            &owner.write(server.put(&path)).json(&stale_input).await,
            409,
        );
        assert!(matches!(
            serde_json::from_value::<LibraryErrorDetails>(e.details.unwrap()).unwrap(),
            LibraryErrorDetails::IdempotencyConflict { .. }
        ));
        let bad_submit = owner
            .write(server.post(&format!("{}/{}/submit", base(app), draft.entry.id)))
            .json(&SubmitLibraryDraftRequest {
                mutation_id: Uuid::new_v4(),
                expected_revision: draft.entry.revision,
            })
            .await;
        let e = failure(&bad_submit, 422);
        assert!(e.details.is_some());
        let mut definition: TestDefinition = serde_json::from_str(include_str!(
            "../../../contracts/fixtures/execution/persistence-case.json"
        ))
        .unwrap();
        let TestDefinition::Case(ref mut c) = definition else {
            panic!()
        };
        c.key = draft.entry.key.clone();
        c.version = 1;
        let good = SaveLibraryDraftRequest {
            mutation_id: Uuid::new_v4(),
            expected_revision: draft.entry.revision,
            definition: definition.into(),
        };
        let r = owner.write(server.put(&path)).json(&good).await;
        r.assert_status_ok();
        draft = r.json();
        assert!(draft.issues.is_empty());
        let LibraryDraftDefinition::Case(c) = &draft.definition else {
            panic!()
        };
        assert_eq!(c.provenance, "user_authored");
        let submit_input = SubmitLibraryDraftRequest {
            mutation_id: Uuid::new_v4(),
            expected_revision: draft.entry.revision,
        };
        let submit_path = format!("{}/{}/submit", base(app), draft.entry.id);
        let r = owner
            .write(server.post(&submit_path))
            .json(&submit_input)
            .await;
        r.assert_status(StatusCode::CREATED);
        let version = r.json::<LibraryVersionResponse>();
        assert_eq!(version.review_state, LibraryReviewState::InReview);
        failure(&owner.read(server.get(&path)).await, 404);
        let replay = owner
            .write(server.post(&submit_path))
            .json(&submit_input)
            .await;
        replay.assert_status_ok();
        assert_eq!(replay.json::<LibraryVersionResponse>(), version);
        let fork = ForkLibraryDraftRequest {
            mutation_id: Uuid::new_v4(),
            expected_revision: version.entry.revision,
            source_version_id: version.version.id,
        };
        let r = owner.write(server.post(&path)).json(&fork).await;
        r.assert_status(StatusCode::CREATED);
        let forked = r.json::<LibraryDraftResponse>();
        assert_eq!(forked.definition.identity().2, 2);
        assert_eq!(
            defs::get(&ctx.db, app, version.version.id).await.unwrap(),
            version.version
        );
        let imported = defs::import(
            &ctx,
            owner.user,
            DefinitionImport {
                app_id: app,
                definition: forked.definition.published().unwrap(),
            },
        )
        .await;
        assert_eq!(imported.unwrap_err().status, StatusCode::CONFLICT);
    })
    .await;
}
#[tokio::test]
async fn review_permissions_rejection_archive_and_default_do_not_rewrite_history() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let (owner, app, profile) = setup(&server, &ctx).await;
        let draft = create(&server, &owner, app, DefinitionKind::Case, Some(profile)).await;
        let version = submit(&server, &owner, app, &draft).await;
        let review_path = format!(
            "{}/{}/versions/{}/review",
            base(app),
            version.entry.id,
            version.version.id
        );
        let mut input = ReviewLibraryVersionRequest {
            mutation_id: Uuid::new_v4(),
            expected_revision: version.entry.revision,
            content_hash: version.version.content_hash.clone(),
            purpose: ApprovalPurpose::Business,
            decision: LibraryReviewDecision::Approve,
            reason: None,
        };
        failure(
            &owner.write(server.post(&review_path)).json(&input).await,
            403,
        );
        defs::grant(&ctx, owner.user, app, owner.user, ApprovalPurpose::Business)
            .await
            .unwrap();
        let r = owner.write(server.post(&review_path)).json(&input).await;
        r.assert_status_ok();
        let business = r.json::<LibraryVersionResponse>();
        assert_eq!(business.review_state, LibraryReviewState::InReview);
        input.mutation_id = Uuid::new_v4();
        input.expected_revision = business.entry.revision;
        failure(
            &owner.write(server.post(&review_path)).json(&input).await,
            409,
        );
        defs::grant(
            &ctx,
            owner.user,
            app,
            owner.user,
            ApprovalPurpose::Executability,
        )
        .await
        .unwrap();
        input.purpose = ApprovalPurpose::Executability;
        input.decision = LibraryReviewDecision::NeedsInput;
        failure(
            &owner.write(server.post(&review_path)).json(&input).await,
            422,
        );
        input.reason = Some("Make the checkpoint observable".into());
        let r = owner.write(server.post(&review_path)).json(&input).await;
        r.assert_status_ok();
        let rejected = r.json::<LibraryVersionResponse>();
        assert_eq!(rejected.review_state, LibraryReviewState::NeedsInput);
        assert_eq!(rejected.version.approvals.len(), 1);
        assert!(defs::approve(
            &ctx,
            owner.user,
            app,
            version.version.id,
            &version.version.content_hash,
            ApprovalPurpose::Executability
        )
        .await
        .is_err());
        let path = format!("{}/{}/draft", base(app), version.entry.id);
        let r = owner
            .write(server.post(&path))
            .json(&ForkLibraryDraftRequest {
                mutation_id: Uuid::new_v4(),
                expected_revision: rejected.entry.revision,
                source_version_id: rejected.version.id,
            })
            .await;
        r.assert_status(StatusCode::CREATED);
        let candidate = submit(&server, &owner, app, &r.json()).await;
        assert!(candidate.version.approvals.is_empty());
        let approved = approve(&server, &ctx, &owner, app, candidate).await;
        let mut plan = create(&server, &owner, app, DefinitionKind::Plan, None).await;
        let LibraryDraftDefinition::Plan(ref mut p) = plan.definition else {
            panic!()
        };
        p.title = "Release".into();
        p.profile_id = Some(profile);
        p.budget.duration_seconds = 1800;
        p.cases = vec![CaseSelection {
            case_version_id: approved.version.id,
            data_variant: "default".into(),
            required: true,
        }];
        let r = owner
            .write(server.put(&format!("{}/{}/draft", base(app), plan.entry.id)))
            .json(&SaveLibraryDraftRequest {
                mutation_id: Uuid::new_v4(),
                expected_revision: plan.entry.revision,
                definition: plan.definition,
            })
            .await;
        r.assert_status_ok();
        plan = r.json();
        let candidate = submit(&server, &owner, app, &plan).await;
        let plan = approve(&server, &ctx, &owner, app, candidate).await;
        let default_path = format!("/api/apps/{app}/default-test-plan");
        assert_eq!(
            owner
                .read(server.get(&default_path))
                .await
                .json::<DefaultPlanResponse>()
                .plan_version_id,
            None
        );
        let input = SetDefaultPlanRequest {
            mutation_id: Uuid::new_v4(),
            expected_revision: 0,
            plan_version_id: plan.version.id,
        };
        let r = owner.write(server.put(&default_path)).json(&input).await;
        r.assert_status_ok();
        let default = r.json::<DefaultPlanResponse>();
        assert_eq!(
            owner
                .write(server.put(&default_path))
                .json(&input)
                .await
                .json::<DefaultPlanResponse>(),
            default
        );
        let archive = owner
            .write(server.post(&format!("{}/{}/archive", base(app), approved.entry.id)))
            .json(&ArchiveLibraryEntryRequest {
                mutation_id: Uuid::new_v4(),
                expected_revision: approved.entry.revision,
                archived: true,
            })
            .await;
        archive.assert_status_ok();
        let TestDefinition::Plan(p) = &plan.version.definition else {
            panic!()
        };
        assert!(defs::resolve(&ctx.db, app, p).await.is_err());
        assert_eq!(
            defs::get(&ctx.db, app, approved.version.id).await.unwrap(),
            approved.version
        );
        assert_eq!(library::default_plan(&ctx.db, app).await.unwrap(), default);
        assert!(library::options(&ctx.db, owner.user, app)
            .await
            .unwrap()
            .approved_versions
            .iter()
            .all(|v| v.version.id != approved.version.id));
    })
    .await;
}
#[tokio::test]
async fn concurrent_saves_scope_csrf_and_actual_route_contracts() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let (owner, app, _) = setup(&server, &ctx).await;
        let foreign = login(&server, &ctx).await;
        let draft = create(&server, &owner, app, DefinitionKind::Suite, None).await;
        let path = format!("{}/{}/draft", base(app), draft.entry.id);
        failure(&foreign.read(server.get(&path)).await, 404);
        let input = SaveLibraryDraftRequest {
            mutation_id: Uuid::new_v4(),
            expected_revision: draft.entry.revision,
            definition: draft.definition.clone(),
        };
        failure(&owner.read(server.put(&path)).json(&input).await, 403);
        let second = SaveLibraryDraftRequest {
            mutation_id: Uuid::new_v4(),
            ..input.clone()
        };
        let (a, b) = tokio::join!(
            async { owner.write(server.put(&path)).json(&input).await },
            async { owner.write(server.put(&path)).json(&second).await }
        );
        let mut statuses = [a.status_code().as_u16(), b.status_code().as_u16()];
        statuses.sort();
        assert_eq!(statuses, [200, 409]);
        let spec = serde_json::to_value(mobile_qa_contracts::browser::openapi()).unwrap();
        for &(method, path, operation, status) in mobile_qa_contracts::test_library_api::OPERATIONS
        {
            let op = &spec["paths"][path][method];
            assert_eq!(op["operationId"], operation);
            assert!(
                op["responses"][status.to_string()]["content"]["application/json"]["schema"]
                    .is_object()
            );
            assert_eq!(
                op["responses"]["default"]["content"]["application/json"]["schema"]["$ref"],
                "#/components/schemas/ApiError"
            );
            let (request_schema, response_schema) = match operation {
                "listTestLibrary" => (None, "LibraryListResponse"),
                "getTestLibraryOptions" => (None, "LibraryOptionsResponse"),
                "createTestLibraryEntry" => {
                    (Some("CreateLibraryEntryRequest"), "LibraryDraftResponse")
                }
                "getTestLibraryEntry" => (None, "LibraryEntryResponse"),
                "getTestLibraryDraft" => (None, "LibraryDraftResponse"),
                "forkTestLibraryDraft" => (Some("ForkLibraryDraftRequest"), "LibraryDraftResponse"),
                "saveTestLibraryDraft" => (Some("SaveLibraryDraftRequest"), "LibraryDraftResponse"),
                "submitTestLibraryDraft" => {
                    (Some("SubmitLibraryDraftRequest"), "LibraryVersionResponse")
                }
                "listTestLibraryVersions" => (None, "LibraryVersionListResponse"),
                "getTestLibraryVersion" => (None, "LibraryVersionResponse"),
                "reviewTestLibraryVersion" => (
                    Some("ReviewLibraryVersionRequest"),
                    "LibraryVersionResponse",
                ),
                "archiveTestLibraryEntry" => {
                    (Some("ArchiveLibraryEntryRequest"), "LibraryEntryResponse")
                }
                "getDefaultTestPlan" => (None, "DefaultPlanResponse"),
                "setDefaultTestPlan" => (Some("SetDefaultPlanRequest"), "DefaultPlanResponse"),
                _ => panic!("Operation lacks a real-handler agreement case"),
            };
            assert_eq!(
                op["responses"][status.to_string()]["content"]["application/json"]["schema"]
                    ["$ref"],
                format!("#/components/schemas/{response_schema}")
            );
            if let Some(request_schema) = request_schema {
                assert_eq!(
                    op["requestBody"]["content"]["application/json"]["schema"]["$ref"],
                    format!("#/components/schemas/{request_schema}")
                );
            } else {
                assert!(op.get("requestBody").is_none());
            }

            let path = path
                .replace("{app_id}", &app.to_string())
                .replace("{entry_id}", &draft.entry.id.to_string())
                .replace("{version_id}", &Uuid::new_v4().to_string());
            let req = match method {
                "get" => server.get(&path),
                "post" => server.post(&path),
                "put" => server.put(&path),
                _ => panic!(),
            };
            // Every declared route reaches Session rather than a 404/405 fallback.
            failure(&req.json(&serde_json::json!({})).await, 401);
        }
        let list = owner.read(server.get(&base(app))).await;
        list.assert_status_ok();
        assert_eq!(list.json::<LibraryListResponse>().items.len(), 1);
        let opt = owner
            .read(server.get(&format!("{}/options", base(app))))
            .await;
        opt.assert_status_ok();
        assert!(
            !opt.json::<LibraryOptionsResponse>()
                .capabilities
                .can_review_business
        );
        let entry = owner
            .read(server.get(&format!("{}/{}", base(app), draft.entry.id)))
            .await;
        entry.assert_status_ok();
        let history = owner
            .read(server.get(&format!("{}/{}/versions", base(app), draft.entry.id)))
            .await;
        history.assert_status_ok();
        assert!(history
            .json::<LibraryVersionListResponse>()
            .items
            .is_empty());
    })
    .await;
}

#[tokio::test]
async fn migration_backfill_preserves_legacy_ids_hashes_approvals_and_reports() {
    use migration::{MigrationTrait, SchemaManager};
    use sea_orm::{ConnectionTrait, TransactionTrait};
    let _guard = DATABASE_BOOT.lock().await;
    request::<App,_,_>(|_server,ctx|async move {
        let tx=ctx.db.begin().await.unwrap();
        // A transaction-local schema keeps upgrade fixtures away from other tests
        // and is rolled back even if an assertion fails.
        let schema=format!("library_upgrade_{}",Uuid::new_v4().simple());
        tx.execute_unprepared(&format!("CREATE SCHEMA {schema}; SET LOCAL search_path TO {schema};")).await.unwrap();
        tx.execute_unprepared("CREATE TABLE users(id UUID PRIMARY KEY); CREATE TABLE apps(id UUID PRIMARY KEY);
          CREATE TABLE execution_definitions(id UUID PRIMARY KEY, app_id UUID, kind TEXT,logical_key TEXT,version INTEGER,content_hash TEXT,payload JSONB,author_id UUID,created_at TIMESTAMPTZ);
          CREATE TABLE execution_approvals(definition_id UUID,purpose TEXT,actor_id UUID,content_hash TEXT,approved_at TIMESTAMPTZ);
          CREATE TABLE execution_profiles(id UUID,app_id UUID);
          CREATE TABLE execution_runs(id UUID,manifest JSONB);").await.unwrap();
        let actor=Uuid::new_v4();let app=Uuid::new_v4();let case=Uuid::new_v4();let plan=Uuid::new_v4();let profile=Uuid::new_v4();
        exec(&tx,"INSERT INTO users VALUES($1)",vec![actor.into()]).await.unwrap();
        exec(&tx,"INSERT INTO apps VALUES($1)",vec![app.into()]).await.unwrap();
        exec(&tx,"INSERT INTO execution_profiles VALUES($1,$2)",vec![profile.into(),app.into()]).await.unwrap();
        let payload:serde_json::Value=serde_json::from_str(include_str!("../../../contracts/fixtures/execution/persistence-case.json")).unwrap();
        exec(&tx,"INSERT INTO execution_definitions VALUES($1,
        $2,'case','TASK-PERSIST',1,'unchanged-case-hash', $3, $4, now())",vec![case.into(),app.into(),payload.clone().into(),actor.into()]).await.unwrap();
        let plan_payload=serde_json::json!({"kind":"plan","content":{"profile_id":profile,"cases":[{"case_version_id":case}],"suite_version_ids":[]}});
        exec(&tx,"INSERT INTO execution_definitions VALUES($1,$2,'plan','release',3,'unchanged-plan-hash',$3,$4,now())",vec![plan.into(),app.into(),plan_payload.into(),actor.into()]).await.unwrap();
        for (id,digest) in [(case,"unchanged-case-hash"),(plan,"unchanged-plan-hash")] {
            for purpose in ["business","executability"] {exec(&tx,"INSERT INTO execution_approvals VALUES($1,$2,$3,$4,now())",vec![id.into(),purpose.into(),actor.into(),digest.into()]).await.unwrap();}
        }
        let manifest=serde_json::json!({"plan_version_id":plan,"case_version_id":case,"historical":"unchanged"});
        exec(&tx,"INSERT INTO execution_runs VALUES($1,$2)",vec![Uuid::new_v4().into(),manifest.clone().into()]).await.unwrap();
        migration::m20260912_000005_test_library::Migration.up(&SchemaManager::new(&tx)).await.unwrap();
        let linked=one(&tx,"SELECT e.next_version, v.review_state, d.payload, d.content_hash FROM test_library_entries e
        JOIN test_library_versions v ON v.entry_id=e.id JOIN execution_definitions d ON
        d.id=v.definition_id WHERE d.id=$1",vec![case.into()]).await.unwrap();
        assert_eq!(field::<i32>(&linked,"next_version").unwrap(),2);assert_eq!(field::<String>(&linked,"review_state").unwrap(),"approved");
        assert_eq!(field::<String>(&linked,"content_hash").unwrap(),"unchanged-case-hash");assert_eq!(field::<serde_json::Value>(&linked,"payload").unwrap(),payload);
        assert_eq!(library::default_plan(&tx,app).await.unwrap().plan_version_id,Some(plan));
        assert_eq!(rows(&tx,"SELECT * FROM test_library_review_events",vec![]).await.unwrap().len(),4);
        assert_eq!(field::<serde_json::Value>(&one(&tx,"SELECT manifest FROM execution_runs",vec![]).await.unwrap(),"manifest").unwrap(),manifest);
        tx.rollback().await.unwrap();
    }).await;
}

#[tokio::test]
async fn members_can_author_but_review_grants_are_independent_and_revocation_blocks_replay() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App,_,_>(|server,ctx|async move {
        let (owner,app,profile)=setup(&server,&ctx).await;
        let reviewer=login(&server,&ctx).await;
        exec(&ctx.db,"INSERT INTO memberships(id,user_id,organization_id,role,active) VALUES($1,$2,$3,'member',true)",vec![Uuid::new_v4().into(),reviewer.user.into(),owner.org.into()]).await.unwrap();
        exec(&ctx.db,"INSERT INTO app_memberships(id,user_id,organization_id,app_id) VALUES($1,$2,$3,$4)",vec![Uuid::new_v4().into(),reviewer.user.into(),owner.org.into(),app.into()]).await.unwrap();
        let draft=create(&server,&reviewer,app,DefinitionKind::Case,Some(profile)).await;
        assert!(draft.entry.capabilities.can_edit);assert!(!draft.entry.capabilities.can_archive);
        let mut candidate=submit(&server,&reviewer,app,&draft).await;
        defs::grant(&ctx,owner.user,app,reviewer.user,ApprovalPurpose::Business).await.unwrap();
        defs::grant(&ctx,owner.user,app,owner.user,ApprovalPurpose::Executability).await.unwrap();
        let path=format!("{}/{}/versions/{}/review",base(app),candidate.entry.id,candidate.version.id);
        let business=ReviewLibraryVersionRequest {mutation_id:Uuid::new_v4(),expected_revision:candidate.entry.revision,content_hash:candidate.version.content_hash.clone(),purpose:ApprovalPurpose::Business,decision:LibraryReviewDecision::Approve,reason:None};
        let r=reviewer.write(server.post(&path)).json(&business).await;r.assert_status_ok();candidate=r.json();
        let execution=ReviewLibraryVersionRequest {mutation_id:Uuid::new_v4(),expected_revision:candidate.entry.revision,purpose:ApprovalPurpose::Executability,..business.clone()};
        failure(&reviewer.write(server.post(&path)).json(&execution).await,403);
        let r=owner.write(server.post(&path)).json(&execution).await;r.assert_status_ok();candidate=r.json();
        assert_eq!(candidate.review_state,LibraryReviewState::Approved);assert_ne!(candidate.version.approvals[0].actor_id,candidate.version.approvals[1].actor_id);
        exec(&ctx.db,"UPDATE memberships SET active=false WHERE organization_id=$1 AND user_id=$2",vec![owner.org.into(),reviewer.user.into()]).await.unwrap();
        failure(&reviewer.write(server.post(&path)).json(&business).await,404);
    }).await;
}

#[tokio::test]
async fn duplicate_pins_report_conflicts_and_new_approvals_do_not_move_the_default() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let (owner, app, profile) = setup(&server, &ctx).await;
        let draft = create(&server, &owner, app, DefinitionKind::Case, Some(profile)).await;
        let candidate = submit(&server, &owner, app, &draft).await;
        let case = approve(&server, &ctx, &owner, app, candidate).await;
        let mut draft = create(&server, &owner, app, DefinitionKind::Plan, Some(profile)).await;
        let selection = CaseSelection {
            case_version_id: case.version.id,
            data_variant: "default".into(),
            required: true,
        };
        let LibraryDraftDefinition::Plan(ref mut p) = draft.definition else {
            panic!()
        };
        p.title = "Release".into();
        p.cases = vec![
            selection.clone(),
            CaseSelection {
                required: false,
                ..selection.clone()
            },
        ];
        let path = format!("{}/{}/draft", base(app), draft.entry.id);
        let r = owner
            .write(server.put(&path))
            .json(&SaveLibraryDraftRequest {
                mutation_id: Uuid::new_v4(),
                expected_revision: draft.entry.revision,
                definition: draft.definition,
            })
            .await;
        r.assert_status_ok();
        draft = r.json();
        assert!(draft
            .issues
            .iter()
            .any(|i| i.code == LibraryIssueCode::ConflictingSelection));
        failure(
            &owner
                .write(server.post(&format!("{}/{}/submit", base(app), draft.entry.id)))
                .json(&SubmitLibraryDraftRequest {
                    mutation_id: Uuid::new_v4(),
                    expected_revision: draft.entry.revision,
                })
                .await,
            409,
        );
        let LibraryDraftDefinition::Plan(ref mut p) = draft.definition else {
            panic!()
        };
        p.cases = vec![selection.clone(), selection];
        let r = owner
            .write(server.put(&path))
            .json(&SaveLibraryDraftRequest {
                mutation_id: Uuid::new_v4(),
                expected_revision: draft.entry.revision,
                definition: draft.definition,
            })
            .await;
        r.assert_status_ok();
        draft = r.json();
        assert_eq!(draft.coverage.cases.len(), 1);
        let candidate = submit(&server, &owner, app, &draft).await;
        let first = approve(&server, &ctx, &owner, app, candidate).await;
        let default_path = format!("/api/apps/{app}/default-test-plan");
        owner
            .write(server.put(&default_path))
            .json(&SetDefaultPlanRequest {
                mutation_id: Uuid::new_v4(),
                expected_revision: 0,
                plan_version_id: first.version.id,
            })
            .await
            .assert_status_ok();
        let r = owner
            .write(server.post(&path))
            .json(&ForkLibraryDraftRequest {
                mutation_id: Uuid::new_v4(),
                expected_revision: first.entry.revision,
                source_version_id: first.version.id,
            })
            .await;
        r.assert_status(StatusCode::CREATED);
        let draft = r.json();
        let candidate = submit(&server, &owner, app, &draft).await;
        let newer = approve(&server, &ctx, &owner, app, candidate).await;
        assert_ne!(newer.version.id, first.version.id);
        assert_eq!(
            owner
                .read(server.get(&default_path))
                .await
                .json::<DefaultPlanResponse>()
                .plan_version_id,
            Some(first.version.id)
        );
        let history = owner
            .read(server.get(&format!("{}/{}/versions", base(app), first.entry.id)))
            .await
            .json::<LibraryVersionListResponse>();
        assert_eq!(history.items[0].version.id, newer.version.id);
        let detail = owner
            .read(server.get(&format!(
                "{}/{}/versions/{}",
                base(app),
                first.entry.id,
                first.version.id
            )))
            .await;
        detail.assert_status_ok();
        assert_eq!(
            detail.json::<LibraryVersionResponse>().version,
            first.version
        );
    })
    .await;
}
