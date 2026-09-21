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
        model: None,
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
async fn save(
    server: &TestServer,
    owner: &Login,
    app: Uuid,
    draft: &LibraryDraftResponse,
) -> LibraryDraftResponse {
    let response = owner
        .write(server.put(&format!("{}/{}/draft", base(app), draft.entry.id)))
        .json(&SaveLibraryDraftRequest {
            mutation_id: Uuid::new_v4(),
            expected_revision: draft.entry.revision,
            definition: draft.definition.clone(),
        })
        .await;
    response.assert_status_ok();
    response.json()
}
#[tokio::test]
async fn save_is_atomic_editable_idempotent_and_preserves_versions() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let (owner, app, profile) = setup(&server, &ctx).await;
        let mut draft = create(&server, &owner, app, DefinitionKind::Case, Some(profile)).await;
        let path = format!("{}/{}/draft", base(app), draft.entry.id);
        let input = SaveLibraryDraftRequest {
            mutation_id: Uuid::new_v4(),
            expected_revision: draft.entry.revision,
            definition: draft.definition.clone(),
        };
        let response = owner.write(server.put(&path)).json(&input).await;
        response.assert_status_ok();
        draft = response.json();
        let first = draft.saved_version_id.unwrap();
        let frozen = defs::get(&ctx.db, app, first).await.unwrap();
        assert!(frozen.approvals.is_empty());
        assert_eq!(
            owner
                .write(server.put(&path))
                .json(&input)
                .await
                .json::<LibraryDraftResponse>(),
            draft
        );
        let stale = SaveLibraryDraftRequest {
            mutation_id: Uuid::new_v4(),
            ..input.clone()
        };
        assert_eq!(
            failure(&owner.write(server.put(&path)).json(&stale).await, 409).code,
            "stale_revision"
        );
        let conflict = SaveLibraryDraftRequest {
            expected_revision: draft.entry.revision,
            ..input
        };
        assert_eq!(
            failure(&owner.write(server.put(&path)).json(&conflict).await, 409).code,
            "idempotency_conflict"
        );
        draft = save(&server, &owner, app, &draft).await;
        assert_eq!(draft.saved_version_id, Some(first));
        // A missing required target can be saved but must never point at the older executable snapshot.
        let complete = draft.definition.clone();
        if let LibraryDraftDefinition::Case(c) = &mut draft.definition {
            c.actions.clear();
        }
        draft = save(&server, &owner, app, &draft).await;
        assert!(draft.saved_version_id.is_none());
        assert!(!draft.issues.is_empty());
        assert_eq!(
            owner
                .read(server.get(&path))
                .await
                .json::<LibraryDraftResponse>(),
            draft
        );
        draft.definition = complete;
        if let LibraryDraftDefinition::Case(c) = &mut draft.definition {
            c.title = "New saved title".into();
        }
        draft = save(&server, &owner, app, &draft).await;
        assert_ne!(draft.saved_version_id, Some(first));
        assert_eq!(draft.definition.identity().2, 2);
        assert_eq!(defs::get(&ctx.db, app, first).await.unwrap(), frozen);
        assert_eq!(
            library::versions(&ctx.db, owner.user, app, draft.entry.id, None)
                .await
                .unwrap()
                .items
                .len(),
            2
        );
        // No legacy write route or reviewer provisioning remains.
        for suffix in [
            "submit",
            "versions/00000000-0000-0000-0000-000000000000/review",
        ] {
            let r = owner
                .write(server.post(&format!("{}/{}/{suffix}", base(app), draft.entry.id)))
                .json(&serde_json::json!({}))
                .await;
            assert!(matches!(r.status_code().as_u16(), 404 | 405));
        }
    })
    .await;
}
#[tokio::test]
async fn members_save_cases_suites_and_plans_without_grants_and_pins_stay_fixed() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App,_,_>(|server,ctx| async move {
        let (owner,app,profile)=setup(&server,&ctx).await;
        let member=login(&server,&ctx).await;
        exec(&ctx.db,"INSERT INTO memberships(id,user_id,organization_id,role,active) VALUES($1,$2,$3,'member',true)",vec![Uuid::new_v4().into(),member.user.into(),owner.org.into()]).await.unwrap();
        exec(&ctx.db,"INSERT INTO app_memberships(id,user_id,organization_id,app_id) VALUES($1,$2,$3,$4)",vec![Uuid::new_v4().into(),member.user.into(),owner.org.into(),app.into()]).await.unwrap();
        let c=create(&server,&member,app,DefinitionKind::Case,Some(profile)).await;
        let mut c=save(&server,&member,app,&c).await;
        let case_id=c.saved_version_id.unwrap();
        let selection=CaseSelection{case_version_id:case_id,data_variant:"default".into(),required:true};
        let mut suite=create(&server,&member,app,DefinitionKind::Suite,None).await;
        if let LibraryDraftDefinition::Suite(s)=&mut suite.definition{s.title="Suite".into();s.cases=vec![selection.clone()];}
        suite=save(&server,&member,app,&suite).await;
        let mut plan=create(&server,&member,app,DefinitionKind::Plan,Some(profile)).await;
        if let LibraryDraftDefinition::Plan(p)=&mut plan.definition{p.title="Plan".into();p.suite_version_ids=vec![suite.saved_version_id.unwrap()];p.cases=vec![selection.clone()];}
        plan=save(&server,&member,app,&plan).await;
        let first=plan.saved_version_id.unwrap();
        let frozen=defs::get(&ctx.db,app,first).await.unwrap();
        let TestDefinition::Plan(p)=&frozen.definition else {panic!()};
        assert_eq!(defs::resolve(&ctx.db,app,p).await.unwrap().len(),1);
        let default_path=format!("/api/apps/{app}/default-test-plan");
        let default=SetDefaultPlanRequest{mutation_id:Uuid::new_v4(),expected_revision:0,plan_version_id:first};
        failure(&member.write(server.put(&default_path)).json(&default).await,403);
        owner.write(server.put(&default_path)).json(&default).await.assert_status_ok();
        if let LibraryDraftDefinition::Case(c)=&mut c.definition{c.title="Changed case".into();}
        c=save(&server,&member,app,&c).await;
        if let LibraryDraftDefinition::Plan(p)=&mut plan.definition{p.title="Changed plan".into();}
        plan=save(&server,&member,app,&plan).await;assert_ne!(plan.saved_version_id,Some(first));
        assert_eq!(library::default_plan(&ctx.db,app).await.unwrap().plan_version_id,Some(first));
        assert_eq!(defs::resolve(&ctx.db,app,p).await.unwrap()[0].definition_id,case_id);
        // Technical conflicts stay visible on an incomplete saved edit.
        if let LibraryDraftDefinition::Plan(p)=&mut plan.definition{p.cases.push(CaseSelection{required:false,..selection});}
        plan=save(&server,&member,app,&plan).await;
        assert!(plan.saved_version_id.is_none());assert!(plan.issues.iter().any(|i|i.code==LibraryIssueCode::ConflictingSelection));
        owner.write(server.post(&format!("{}/{}/archive",base(app),c.entry.id))).json(&ArchiveLibraryEntryRequest{mutation_id:Uuid::new_v4(),expected_revision:c.entry.revision,archived:true}).await.assert_status_ok();
        assert!(defs::resolve(&ctx.db,app,p).await.is_err());
        assert_eq!(defs::get(&ctx.db,app,first).await.unwrap(),frozen);
        assert!(rows(&ctx.db,"SELECT * FROM execution_reviewer_grants WHERE app_id=$1",vec![app.into()]).await.unwrap().is_empty());
        assert!(rows(&ctx.db,"SELECT a.* FROM execution_approvals a JOIN execution_definitions d ON d.id=a.definition_id WHERE d.app_id=$1",vec![app.into()]).await.unwrap().is_empty());
    }).await;
}
#[tokio::test]
async fn legacy_review_states_are_editable_and_old_receipts_fail_closed() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App,_,_>(|server,ctx| async move {
        let (owner,app,profile)=setup(&server,&ctx).await;
        for state in ["in_review","needs_input","rejected","approved"] {
            let initial=create(&server,&owner,app,DefinitionKind::Case,Some(profile)).await;
            let saved=save(&server,&owner,app,&initial).await;
            let id=saved.saved_version_id.unwrap();
            exec(&ctx.db,"UPDATE test_library_versions SET legacy_review_state=$2 WHERE definition_id=$1",vec![id.into(),state.into()]).await.unwrap();
            exec(&ctx.db,"DELETE FROM test_library_drafts WHERE entry_id=$1",vec![saved.entry.id.into()]).await.unwrap();
            library::admitted(&ctx.db,app,id).await.unwrap();
            let current=library::draft(&ctx.db,owner.user,app,saved.entry.id).await.unwrap();
            assert_eq!(current.saved_version_id,Some(id));
            let again=save(&server,&owner,app,&current).await;assert_eq!(again.saved_version_id,Some(id));
        }
        let draft=create(&server,&owner,app,DefinitionKind::Case,None).await;
        let mutation_id=Uuid::new_v4();
        let input=SaveLibraryDraftRequest{mutation_id,expected_revision:draft.entry.revision,definition:draft.definition.clone()};
        let old=serde_json::json!({"operation":"save","request":[draft.entry.id,input]});
        let fingerprint=hash(serde_json::to_vec(&old).unwrap());
        exec(&ctx.db,"INSERT INTO test_library_mutations(app_id,actor_id,mutation_id,fingerprint,response) VALUES($1,$2,$3,$4,$5)",vec![app.into(),owner.user.into(),mutation_id.into(),fingerprint.into(),serde_json::json!({"legacy":"receipt preserved"}).into()]).await.unwrap();
        let path=format!("{}/{}/draft",base(app),draft.entry.id);
        assert_eq!(failure(&owner.write(server.put(&path)).json(&input).await,409).code,"idempotency_conflict");
        // Authorization is rechecked even for an otherwise valid replay.
        let fresh=SaveLibraryDraftRequest{mutation_id:Uuid::new_v4(),..input};
        owner.write(server.put(&path)).json(&fresh).await.assert_status_ok();
        exec(&ctx.db,"UPDATE memberships SET active=false WHERE organization_id=$1 AND user_id=$2",vec![owner.org.into(),owner.user.into()]).await.unwrap();
        failure(&owner.write(server.put(&path)).json(&fresh).await,404);
    }).await;
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
                "saveTestLibraryDraft" => (Some("SaveLibraryDraftRequest"), "LibraryDraftResponse"),
                "listTestLibraryVersions" => (None, "LibraryVersionListResponse"),
                "getTestLibraryVersion" => (None, "LibraryVersionResponse"),
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
        assert!(opt.json::<LibraryOptionsResponse>().capabilities.can_edit);
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
        migration::m20260920_000008_save_without_reviews::Migration.up(&SchemaManager::new(&tx)).await.unwrap();
        let linked=one(&tx,"SELECT e.next_version, v.legacy_review_state, d.payload, d.content_hash FROM test_library_entries e
        JOIN test_library_versions v ON v.entry_id=e.id JOIN execution_definitions d ON
        d.id=v.definition_id WHERE d.id=$1",vec![case.into()]).await.unwrap();
        assert_eq!(field::<i32>(&linked,"next_version").unwrap(),2);assert_eq!(field::<String>(&linked,"legacy_review_state").unwrap(),"approved");
        assert_eq!(field::<String>(&linked,"content_hash").unwrap(),"unchanged-case-hash");assert_eq!(field::<serde_json::Value>(&linked,"payload").unwrap(),payload);
        assert_eq!(library::default_plan(&tx,app).await.unwrap().plan_version_id,Some(plan));
        assert_eq!(rows(&tx,"SELECT * FROM test_library_review_events",vec![]).await.unwrap().len(),4);
        assert_eq!(field::<serde_json::Value>(&one(&tx,"SELECT manifest FROM execution_runs",vec![]).await.unwrap(),"manifest").unwrap(),manifest);
        // The forward migration is reversible only while every row has legacy audit state.
        migration::m20260920_000008_save_without_reviews::Migration.down(&SchemaManager::new(&tx)).await.unwrap();
        migration::m20260920_000008_save_without_reviews::Migration.up(&SchemaManager::new(&tx)).await.unwrap();
        exec(&tx,"UPDATE test_library_versions SET legacy_review_state=NULL WHERE definition_id=$1",vec![case.into()]).await.unwrap();
        assert!(migration::m20260920_000008_save_without_reviews::Migration.down(&SchemaManager::new(&tx)).await.is_err());
        tx.rollback().await.unwrap();
    }).await;
}
