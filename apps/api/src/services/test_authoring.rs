//! Interactive authoring reuses phone leases and library receipts; no parallel queue.
use super::{
    apps,
    execution_store::{conflict, decode, hash, json},
    task_sessions as phones, test_library,
};
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::{
        apps as app_rows, environments, phone_sessions, phone_tasks, test_library_drafts,
        test_library_entries,
    },
};
use loco_rs::app::AppContext;
use mobile_qa_contracts::model_registry::ModelCapability;
use mobile_qa_contracts::{automation::*, execution::*, task_sessions::*, test_library::*};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter, QuerySelect, Set, TransactionTrait,
};
use uuid::Uuid;

pub async fn command(
    ctx: &AppContext,
    user: Uuid,
    id: Uuid,
    input: PhoneCommandRequest,
) -> ApiResult<PhoneSession> {
    let session = phones::authorized(ctx, user, id).await?;
    super::commercial::require_separate_scope(ctx, user, session.app_id).await?;
    input
        .sequence
        .validate(&session.profile.package)
        .map_err(ApiFailure::invalid)?;
    if input.title.trim().is_empty() || input.title.len() > 200 {
        return Err(ApiFailure::invalid("Name this task using 1–200 characters"));
    }
    if input.sequence.uses_ai()
        && session
            .resolved_model
            .as_ref()
            .is_none_or(|model| !model.has(ModelCapability::MinitapNavigation))
    {
        return Err(ApiFailure::invalid(
            "This step needs an AI-enabled device profile",
        ));
    }
    let fingerprint = hash(serde_json::to_vec(&input).map_err(|_| ApiFailure::internal())?);
    enqueue(
        ctx,
        id,
        input.expected_revision,
        input.frame_id,
        fingerprint,
        input.purpose,
        PhoneTask {
            id: input.id,
            goal: input.title,
            control: None,
            state: PhoneTaskState::Queued,
            message: "Waiting to run steps".into(),
            sequence: Some(input.sequence),
            steps: vec![],
            generation: None,
            progress: None,
        },
    )
    .await
}
async fn enqueue(
    ctx: &AppContext,
    id: Uuid,
    revision: u32,
    frame: Option<Uuid>,
    fingerprint: String,
    purpose: Option<mobile_qa_contracts::regression::CommandPurpose>,
    task: PhoneTask,
) -> ApiResult<PhoneSession> {
    let tx = ctx.db.begin().await?;
    let session_row = phone_sessions::Entity::find_by_id(id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let mut s: PhoneSession = decode(session_row.payload.clone())?;
    if let Some(prior) = phone_tasks::Entity::find_by_id(task.id).one(&tx).await? {
        if prior.session_id != id || prior.fingerprint != fingerprint {
            return Err(conflict(
                "Request identity was already used for different content",
            ));
        }
        return phones::detail(&tx, id).await;
    }
    let revision_now = environments::Entity::find()
        .filter(environments::Column::AppId.eq(s.app_id))
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?
        .revision;
    if s.environment_revision != revision_now as u32 {
        return Err(conflict("Environment changed. Open a new session."));
    }
    if ![2, 3, 4, 5, 6].contains(&s.protocol_version) {
        return Err(conflict(
            "Reconnect with the updated device worker to run direct steps",
        ));
    }
    if s.state != PhoneState::Ready || s.revision != revision {
        return Err(conflict(
            "The phone changed or is busy. Refresh before running.",
        ));
    }
    if frame.is_some() && s.frame.as_ref().map(|f| f.id) != frame {
        return Err(conflict("The screen changed. Pick the control again."));
    }
    let count = phone_tasks::Entity::find()
        .filter(phone_tasks::Column::SessionId.eq(id))
        .count(&tx)
        .await?;
    if count >= 100 {
        return Err(ApiFailure::invalid("Open a new session after 100 commands"));
    }
    phone_tasks::ActiveModel {
        id: Set(task.id),
        session_id: Set(id),
        fingerprint: Set(fingerprint),
        payload: Set(json(&task)?),
        purpose: Set(purpose.map(|purpose| match purpose {
            mobile_qa_contracts::regression::CommandPurpose::Trial => "trial".into(),
            mobile_qa_contracts::regression::CommandPurpose::Manual => "manual".into(),
        })),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    s.revision = s
        .revision
        .checked_add(1)
        .ok_or_else(|| conflict("Session revision limit"))?;
    s.state = PhoneState::Acting;
    s.message = "Running your request".into();
    let mut active = session_row.into_active_model();
    active.payload = Set(json(&s)?);
    active.update(&tx).await?;
    tx.commit().await?;
    phones::detail(&ctx.db, id).await
}
pub async fn generate(
    ctx: &AppContext,
    user: Uuid,
    app: Uuid,
    input: GenerateTestsRequest,
) -> ApiResult<PhoneSession> {
    let s = phones::authorized(ctx, user, input.session_id).await?;
    super::commercial::require_separate_scope(ctx, user, app).await?;
    if s.app_id != app {
        return Err(ApiFailure::unauthorized());
    }
    if input.engine != Some(DiscoveryEngine::MinitapV1) {
        return Err(conflict("Refresh this page to explore with AI"));
    }
    if ![3, 4, 5, 6].contains(&s.protocol_version) {
        return Err(conflict("Reconnect with the updated discovery worker"));
    }
    if s.profile.driver != Driver::Minitap
        || s.resolved_model
            .as_ref()
            .is_none_or(|model| !model.has(ModelCapability::MinitapNavigation))
        || s.authoring_model
            .as_ref()
            .or(s.resolved_model.as_ref())
            .is_none_or(|model| !model.has(ModelCapability::StructuredAuthoring))
    {
        return Err(ApiFailure::invalid(
            "AI generation needs a configured model. Templates work without AI.",
        ));
    }
    if input.journey.len() > 4000 {
        return Err(ApiFailure::invalid(
            "Keep the journey under 4,000 characters",
        ));
    }
    if input.allow_writes && input.journey.trim().is_empty() {
        return Err(ApiFailure::invalid(
            "Describe the test-data changes this journey permits",
        ));
    }
    // Reuse is deliberately restricted to the same live session/initial state scope.
    let previous = if let Some(job) = input.reuse_job_id {
        let old = job_detail(ctx, user, app, job).await?;
        let binding = phone_tasks::Entity::find_by_id(job)
            .one(&ctx.db)
            .await?
            .ok_or_else(ApiFailure::missing)?
            .session_id;
        if binding != s.id
            || old.generation.as_ref().is_none_or(|g| {
                g.allow_writes != input.allow_writes
                    || g.journey != input.journey
                    || g.engine != input.engine
            })
        {
            return Err(conflict("Discovery scope changed. Discover again."));
        }
        if old.state != PhoneTaskState::Completed {
            return Err(conflict("Only completed discovery evidence can be reused"));
        }
        old.progress.map(|mut p| {
            p.source_job_id = Some(job);
            p.state = GenerationState::Queued;
            p.proposals.clear();
            p.usage = AuthoringUsage::default();
            p
        })
    } else {
        None
    };
    let fingerprint = hash(serde_json::to_vec(&input).map_err(|_| ApiFailure::internal())?);
    enqueue(
        ctx,
        s.id,
        input.expected_revision,
        None,
        fingerprint,
        None,
        PhoneTask {
            id: input.id,
            goal: format!("Generate {:?} tests", input.category),
            control: None,
            state: PhoneTaskState::Queued,
            message: "Waiting to discover your app".into(),
            sequence: None,
            steps: vec![],
            generation: Some(input),
            progress: previous,
        },
    )
    .await
}
pub async fn job_detail(ctx: &AppContext, user: Uuid, app: Uuid, id: Uuid) -> ApiResult<PhoneTask> {
    let row = phone_tasks::Entity::find_by_id(id)
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let s = phones::authorized(ctx, user, row.session_id).await?;
    if s.app_id != app {
        return Err(ApiFailure::unauthorized());
    }
    let task: PhoneTask = decode(row.payload)?;
    if task.generation.is_none() {
        return Err(ApiFailure::invalid("This is not a generation request"));
    }
    Ok(task)
}
pub async fn cancel(ctx: &AppContext, user: Uuid, app: Uuid, id: Uuid) -> ApiResult<PhoneSession> {
    job_detail(ctx, user, app, id).await?;
    let session = phone_tasks::Entity::find_by_id(id)
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)?
        .session_id;
    phones::stop(ctx, user, session).await
}
fn action(id: &str, command: DirectCommand) -> TestAction {
    TestAction {
        id: id.into(),
        checkpoint_id: id.into(),
        kind: ActionKind::Direct,
        instruction: String::new(),
        command: Some(command),
    }
}
fn target() -> DirectTarget {
    DirectTarget::ResourceId {
        value: String::new(),
    }
}
pub async fn templates(ctx: &AppContext, user: Uuid, app: Uuid) -> ApiResult<TestTemplates> {
    let a = apps::authorized(ctx, user, app).await?;
    let mut items = vec![];
    for (id, title, description, category) in [
        (
            "smoke-open",
            "Smoke — app opens",
            "Pick a landing-screen marker to check the app is ready.",
            CoverageKind::Smoke,
        ),
        (
            "form-submit",
            "Form — submit valid input",
            "Pick an input, submit button and success marker.",
            CoverageKind::HappyPath,
        ),
        (
            "required-field",
            "Validation — required field",
            "Confirm the expected error for submitting empty input.",
            CoverageKind::Validation,
        ),
        (
            "restart-persistence",
            "Persistence — value survives restart",
            "Pick input, save and the retained value after restart.",
            CoverageKind::Persistence,
        ),
    ] {
        let mut actions = if id == "smoke-open" {
            vec![action("ready", DirectCommand::WaitFor { target: target() })]
        } else {
            vec![
                action(
                    "input",
                    DirectCommand::SetText {
                        target: target(),
                        text: if id == "required-field" {
                            String::new()
                        } else {
                            "Buy milk".into()
                        },
                    },
                ),
                action("save", DirectCommand::Tap { target: target() }),
            ]
        };
        if id == "restart-persistence" {
            actions.push(action("restart", DirectCommand::Restart {}));
        }
        let checkpoint = actions
            .last()
            .expect("template has actions")
            .checkpoint_id
            .clone();
        items.push(TestTemplate {
            id: id.into(),
            version: 1,
            title: title.into(),
            description: description.into(),
            category,
            definition: CaseDefinition {
                key: format!("template-{id}"),
                version: 1,
                title: title.into(),
                requirement: String::new(),
                provenance: format!("template:{id}:1"),
                package: a.android_package.clone(),
                adapter: "demo_persistence_v1".into(),
                preconditions: vec![],
                actions,
                checks: vec![ExpectedCheck {
                    id: "result".into(),
                    checkpoint_id: checkpoint,
                    description: "Expected result — pick a control and confirm the value".into(),
                    method: CheckMethod::UiPropertyEqualsV1,
                    resource_id: String::new(),
                    text_filter: String::new(),
                    property: UiProperty::Text,
                    expected: String::new(),
                    ready_resource_id: String::new(),
                    prerequisite_check_ids: vec![],
                    required: true,
                    observation_seconds: 10,
                }],
                budget: ExecutionBudget {
                    duration_seconds: 300,
                    max_steps: 80,
                    artifact_bytes: 16777216,
                },
            },
        });
    }
    Ok(TestTemplates { items })
}
/// Called inside the library's existing app lock and receipt transaction.
pub async fn save(
    db: &impl ConnectionTrait,
    user: Uuid,
    app: Uuid,
    input: SaveAuthoredTestsRequest,
) -> ApiResult<SavedAuthoredTests> {
    if input.tests.is_empty() || input.tests.len() > 5 {
        return Err(ApiFailure::invalid("Save 1–5 selected tests"));
    }
    let caps = test_library::capabilities(db, user, app).await?;
    if !caps.can_edit {
        return Err(ApiFailure::unauthorized());
    }
    let package = app_rows::Entity::find_by_id(app)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?
        .android_package;
    if serde_json::to_vec(&input)
        .map_err(|_| ApiFailure::internal())?
        .len()
        > 1048576
    {
        return Err(ApiFailure::invalid("Draft batch exceeds 1 MiB"));
    }
    let adapter = super::execution_readiness::adapter(db, app, &package).await?;
    let mut proposal_ids = std::collections::BTreeMap::new();
    let source = if let Some(task) = input.source_task_id {
        let row = phone_tasks::Entity::find_by_id(task)
            .one(db)
            .await?
            .ok_or_else(ApiFailure::missing)?;
        let session = phone_sessions::Entity::find_by_id(row.session_id)
            .filter(phone_sessions::Column::AppId.eq(app))
            .filter(phone_sessions::Column::CreatorId.eq(user))
            .one(db)
            .await?
            .ok_or_else(ApiFailure::missing)?;
        let _ = session;
        let t: PhoneTask = decode(row.payload)?;
        if t.generation.is_some() {
            if t.state != PhoneTaskState::Completed {
                return Err(conflict("Generation is not ready"));
            }
            for p in t.progress.as_ref().into_iter().flat_map(|p| &p.proposals) {
                proposal_ids.insert(p.id, p.sequence.clone());
            }
        }
        format!("authored-task:{task}")
    } else {
        "user_authored".into()
    };
    let mut ids = vec![];
    for input in input.tests {
        if input.title.trim().is_empty()
            || input.title.len() > 200
            || input.requirement.len() > 4000
            || input.sequence.actions.len() > 20
            || input.sequence.checks.len() > 20
        {
            return Err(ApiFailure::invalid("Invalid test draft"));
        }
        let provenance = if let Some(template) = &input.template_id {
            if input.proposal_id.is_some()
                || ![
                    "smoke-open",
                    "form-submit",
                    "required-field",
                    "restart-persistence",
                ]
                .contains(&template.as_str())
            {
                return Err(ApiFailure::invalid("Unknown template"));
            }
            format!("template:{template}:1")
        } else if let Some(proposal) = input.proposal_id {
            if !proposal_ids.contains_key(&proposal) {
                return Err(ApiFailure::invalid("Unknown source proposal"));
            }
            format!(
                "{source}:p:{}:{}",
                proposal.simple(),
                if proposal_ids.get(&proposal) == Some(&input.sequence) {
                    "unchanged"
                } else {
                    "user-modified"
                }
            )
        } else {
            if !proposal_ids.is_empty() {
                return Err(ApiFailure::invalid("Choose a source proposal"));
            }
            source.clone()
        };
        if adapter == "android_direct_v1"
            && input
                .sequence
                .actions
                .iter()
                .any(|a| a.kind == ActionKind::Navigate || a.checkpoint_id == "preflight")
        {
            return Err(ApiFailure::invalid(
                "Use direct actions for this app's saved tests",
            ));
        }
        let id = Uuid::new_v4();
        let key = format!("test-{}", id.simple());
        let c = CaseDefinition {
            key: key.clone(),
            version: 1,
            title: input.title,
            requirement: input.requirement,
            provenance,
            package: package.clone(),
            adapter: adapter.clone(),
            preconditions: vec![],
            actions: input.sequence.actions,
            checks: input.sequence.checks,
            budget: ExecutionBudget {
                duration_seconds: 300,
                max_steps: 80,
                artifact_bytes: 16777216,
            },
        };
        let definition = LibraryDraftDefinition::Case(c);
        definition.check_bounds().map_err(ApiFailure::invalid)?;
        test_library_entries::ActiveModel {
            id: Set(id),
            app_id: Set(app),
            kind: Set("case".into()),
            logical_key: Set(key),
            next_version: Set(2),
            actor_id: Set(user),
            ..Default::default()
        }
        .insert(db)
        .await?;
        test_library_drafts::ActiveModel {
            entry_id: Set(id),
            version: Set(1),
            payload: Set(json(&definition)?),
            editor_id: Set(user),
            ..Default::default()
        }
        .insert(db)
        .await?;
        super::test_library_save::save(db, user, app, id, definition).await?;
        ids.push(id);
    }
    Ok(SavedAuthoredTests { entry_ids: ids })
}
