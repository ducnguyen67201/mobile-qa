//! Durable interactive sessions. A lost lease quarantines the phone instead of replaying effects.
use super::{apps, execution_store::*, model_registry, test_definitions, worker_auth::Worker};
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::{
        apps as app_rows, builds, device_slot_bindings, environments, execution_profiles,
        execution_reservations, execution_workers, phone_sessions as phone_rows, phone_tasks,
    },
};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{Duration, Utc};
use loco_rs::app::AppContext;
use mobile_qa_contracts::{
    execution::{Driver, ExecutionProfile},
    model_registry::{ModelCapability, ModelPurpose},
    task_sessions::*,
};
use sea_orm::{
    sea_query::{LockBehavior, LockType, OnConflict},
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use uuid::Uuid;

async fn reconcile(ctx: &AppContext) -> ApiResult<()> {
    let now = Utc::now();
    let expired = phone_rows::Entity::find()
        .filter(
            Condition::any()
                .add(
                    Condition::all()
                        .add(phone_rows::Column::WorkerId.is_not_null())
                        .add(phone_rows::Column::ExpiresAt.lt(now)),
                )
                .add(
                    Condition::all()
                        .add(phone_rows::Column::WorkerId.is_null())
                        .add(phone_rows::Column::Deadline.lt(now)),
                ),
        )
        .all(&ctx.db)
        .await?;
    for row in expired {
        let mut session: PhoneSession = decode(row.payload.clone())?;
        if row.worker_id.is_some()
            && !matches!(session.state, PhoneState::Closed | PhoneState::Quarantined)
        {
            session.state = PhoneState::Quarantined;
            session.message = "Connection lost. The operator must recover this device.".into();
        } else if row.worker_id.is_none() && row.deadline < now {
            session.state = PhoneState::Closed;
        } else {
            continue;
        }
        let mut active = row.into_active_model();
        active.payload = Set(json(&session)?);
        active.update(&ctx.db).await?;
    }
    Ok(())
}
async fn read(db: &impl ConnectionTrait, id: Uuid, lock: bool) -> ApiResult<phone_rows::Model> {
    let query = phone_rows::Entity::find_by_id(id);
    let row = if lock {
        query.lock_exclusive().one(db).await?
    } else {
        query.one(db).await?
    };
    row.ok_or_else(ApiFailure::missing)
}
async fn store(db: &impl ConnectionTrait, s: &PhoneSession) -> ApiResult<()> {
    let mut snapshot = s.clone();
    snapshot.tasks.clear();
    phone_rows::Entity::update_many()
        .col_expr(
            phone_rows::Column::Payload,
            sea_orm::sea_query::Expr::value(json(&snapshot)?),
        )
        .filter(phone_rows::Column::Id.eq(s.id))
        .exec(db)
        .await?;
    Ok(())
}
pub async fn detail(db: &impl ConnectionTrait, id: Uuid) -> ApiResult<PhoneSession> {
    let mut s: PhoneSession = decode(read(db, id, false).await?.payload)?;
    s.tasks = phone_tasks::Entity::find()
        .filter(phone_tasks::Column::SessionId.eq(id))
        .order_by_asc(phone_tasks::Column::CreatedAt)
        .order_by_asc(phone_tasks::Column::Id)
        .all(db)
        .await?
        .into_iter()
        .map(|row| decode(row.payload))
        .collect::<ApiResult<_>>()?;
    Ok(s)
}
pub async fn authorized(ctx: &AppContext, user: Uuid, id: Uuid) -> ApiResult<PhoneSession> {
    reconcile(ctx).await?;
    let r = read(&ctx.db, id, false).await?;
    apps::authorized(ctx, user, r.app_id).await?;
    // Only the session creator controls or views the disposable account state.
    if r.creator_id != user {
        return Err(ApiFailure::new(
            403,
            "session_owner_required",
            "Open your own app session",
        ));
    }
    detail(&ctx.db, id).await
}
pub async fn options(ctx: &AppContext, user: Uuid, app: Uuid) -> ApiResult<PhoneOptions> {
    let a = apps::authorized(ctx, user, app).await?;
    reconcile(ctx).await?;
    let builds: Vec<PhoneBuildChoice> = builds::Entity::find()
        .filter(builds::Column::AppId.eq(app))
        .filter(builds::Column::ValidationState.eq("validated"))
        .order_by_desc(builds::Column::CreatedAt)
        .order_by_desc(builds::Column::Id)
        .limit(50)
        .all(&ctx.db)
        .await?
        .into_iter()
        .map(|row| PhoneBuildChoice {
            id: row.id,
            name: row.original_filename,
        })
        .collect();
    let worker_profiles = execution_workers::Entity::find()
        .filter(execution_workers::Column::AppId.eq(app))
        .filter(execution_workers::Column::Revoked.eq(false))
        .all(&ctx.db)
        .await?
        .into_iter()
        .map(|row| row.profile_id)
        .collect::<std::collections::HashSet<_>>();
    let bound_profiles = device_slot_bindings::Entity::find()
        .filter(device_slot_bindings::Column::AppId.eq(app))
        .all(&ctx.db)
        .await?
        .into_iter()
        .map(|row| row.profile_id)
        .collect::<std::collections::HashSet<_>>();
    let candidates = execution_profiles::Entity::find()
        .filter(execution_profiles::Column::AppId.eq(app))
        .all(&ctx.db)
        .await?
        .into_iter()
        .filter(|row| worker_profiles.contains(&row.id) || bound_profiles.contains(&row.id))
        .map(|row| decode::<ExecutionProfile>(row.payload))
        .collect::<ApiResult<Vec<_>>>()?;
    let mut profiles = Vec::new();
    for profile in candidates.into_iter().filter(|profile| {
        profile.qualified
            && profile.validate().is_ok()
            && matches!(profile.driver, Driver::Minitap | Driver::Direct)
            && profile.package == a.android_package
    }) {
        if profile.driver == Driver::Direct && profile.is_model_free() {
            profiles.push(profile);
            continue;
        }
        let Ok(assignment) =
            model_registry::active_assignment(&ctx.db, app, profile.id, ModelPurpose::Navigation)
                .await
        else {
            continue;
        };
        let Ok(Some(_resolved)) = (if let Some((model, _)) = assignment {
            Ok(Some(model))
        } else {
            model_registry::resolve_for_new_work(
                &ctx.db,
                profile.model.as_ref(),
                &[ModelCapability::MinitapNavigation],
            )
            .await
        }) else {
            continue;
        };
        profiles.push(profile);
    }
    let active_session = phone_rows::Entity::find()
        .filter(phone_rows::Column::AppId.eq(app))
        .filter(phone_rows::Column::CreatorId.eq(user))
        .order_by_desc(phone_rows::Column::CreatedAt)
        .all(&ctx.db)
        .await?
        .into_iter()
        .find_map(|row| {
            decode::<PhoneSession>(row.payload)
                .ok()
                .filter(|session| {
                    !matches!(session.state, PhoneState::Closed | PhoneState::Quarantined)
                })
                .map(|session| session.id)
        });
    let mut blockers = vec![];
    let env = environments::Entity::find()
        .filter(environments::Column::AppId.eq(app))
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if env.account_secret_reference_id.is_some() || env.reset_secret_reference_id.is_some() {
        blockers
            .push("This app needs a verified account and reset setup before connecting.".into());
    }
    if builds.is_empty() {
        blockers.push("Upload an APK to open your app.".into());
    }
    if profiles.is_empty() {
        blockers.push(
            "A device worker needs to be connected for this app. Contact your operator.".into(),
        );
    }
    Ok(PhoneOptions {
        builds,
        profiles,
        active_session,
        blockers,
    })
}
pub async fn open(
    ctx: &AppContext,
    user: Uuid,
    app: Uuid,
    input: OpenPhoneRequest,
) -> ApiResult<PhoneSession> {
    super::commercial::require_separate_scope(ctx, user, app).await?;
    let choices = options(ctx, user, app).await?;
    let fingerprint = hash(serde_json::to_vec(&input).map_err(|_| ApiFailure::internal())?);
    let tx = ctx.db.begin().await?;
    app_rows::Entity::find_by_id(app)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if let Some(row) = phone_rows::Entity::find_by_id(input.id).one(&tx).await? {
        if row.app_id != app || row.creator_id != user || row.fingerprint != fingerprint {
            return Err(conflict("Session request identity was already used"));
        }
        return detail(&tx, input.id).await;
    }
    let active = phone_rows::Entity::find()
        .filter(phone_rows::Column::AppId.eq(app))
        .filter(phone_rows::Column::CreatorId.eq(user))
        .all(&tx)
        .await?
        .into_iter()
        .find(|row| {
            decode::<PhoneSession>(row.payload.clone()).is_ok_and(|session| {
                !matches!(session.state, PhoneState::Closed | PhoneState::Quarantined)
            })
        });
    if let Some(row) = active {
        return detail(&tx, row.id).await;
    }
    let build = input
        .build_id
        .or_else(|| choices.builds.first().map(|b| b.id))
        .ok_or_else(|| ApiFailure::invalid("Upload an APK first"))?;
    let profile_id = input
        .profile_id
        .or_else(|| (choices.profiles.len() == 1).then(|| choices.profiles[0].id))
        .ok_or_else(|| ApiFailure::invalid("Choose a connected device"))?;
    let profile = choices
        .profiles
        .into_iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| ApiFailure::invalid("Device is not available for this app"))?;
    let b = builds::Entity::find_by_id(build)
        .filter(builds::Column::AppId.eq(app))
        .filter(builds::Column::ValidationState.eq("validated"))
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let bytes = b.byte_size;
    if bytes < 1 || bytes > i64::from(profile.max_apk_bytes) {
        return Err(ApiFailure::invalid("Build is too large for this device"));
    }
    let environment_revision = environments::Entity::find()
        .filter(environments::Column::AppId.eq(app))
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?
        .revision;
    let assignment = if profile.driver == Driver::Minitap {
        model_registry::active_assignment(&tx, app, profile.id, ModelPurpose::Navigation).await?
    } else {
        None
    };
    let resolved_model = if let Some((model, _)) = &assignment {
        Some(model.clone())
    } else {
        model_registry::resolve_for_new_work(
            &tx,
            profile.model.as_ref(),
            if profile.driver == Driver::Minitap {
                &[ModelCapability::MinitapNavigation]
            } else {
                &[]
            },
        )
        .await?
    };
    if profile.driver == Driver::Minitap && resolved_model.is_none() {
        return Err(conflict("No active navigation model assignment"));
    }
    let authoring_assignment = if profile.driver == Driver::Minitap {
        model_registry::active_assignment(&tx, app, profile.id, ModelPurpose::StructuredAuthoring)
            .await?
    } else {
        None
    };
    let authoring_model = authoring_assignment
        .as_ref()
        .map(|(model, _)| model.clone())
        .or_else(|| {
            resolved_model
                .as_ref()
                .filter(|model| model.has(ModelCapability::StructuredAuthoring))
                .cloned()
        });
    let s = PhoneSession {
        environment_revision: environment_revision as u32,
        revision: 0,
        protocol_version: 0,
        id: input.id,
        app_id: app,
        build_id: build,
        profile: profile.clone(),
        resolved_model,
        model_assignment_revision: assignment.map(|(_, revision)| revision),
        authoring_model,
        authoring_assignment_revision: authoring_assignment.map(|(_, revision)| revision),
        state: PhoneState::Queued,
        message: "Waiting for a device".into(),
        frame: None,
        tasks: vec![],
    };
    phone_rows::ActiveModel {
        id: Set(s.id),
        app_id: Set(app),
        creator_id: Set(user),
        build_id: Set(build),
        profile_id: Set(profile.id),
        payload: Set(json(&s)?),
        fingerprint: Set(fingerprint),
        build_sha256: Set(b.sha256),
        build_bytes: Set(bytes),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    super::capacity_control::enqueue(&tx, app, profile.id).await?;
    tx.commit().await?;
    super::execution_wakeup::notify(ctx);
    Ok(s)
}
pub async fn task(
    ctx: &AppContext,
    user: Uuid,
    id: Uuid,
    input: PhoneTaskRequest,
) -> ApiResult<PhoneSession> {
    let session = authorized(ctx, user, id).await?;
    super::commercial::require_separate_scope(ctx, user, session.app_id).await?;
    if input.goal.trim().is_empty() || input.goal.len() > 4000 {
        return Err(ApiFailure::invalid(
            "Describe a task using 1–4000 characters",
        ));
    }
    let tx = ctx.db.begin().await?;
    let r = read(&tx, id, true).await?;
    let mut s: PhoneSession = decode(r.payload)?;
    let fingerprint = hash(serde_json::to_vec(&input).map_err(|_| ApiFailure::internal())?);
    if let Some(prior) = phone_tasks::Entity::find_by_id(input.id).one(&tx).await? {
        if prior.session_id != id || prior.fingerprint != fingerprint {
            return Err(conflict("Task request identity was already used"));
        }
        return detail(&tx, id).await;
    }
    if s.resolved_model
        .as_ref()
        .is_none_or(|model| !model.has(ModelCapability::MinitapNavigation))
    {
        return Err(ApiFailure::invalid(
            "Legacy tasks require a model; use structured commands for direct execution",
        ));
    }
    if s.state != PhoneState::Ready {
        return Err(conflict("Wait for the phone to be ready"));
    }
    let task_count = phone_tasks::Entity::find()
        .filter(phone_tasks::Column::SessionId.eq(id))
        .count(&tx)
        .await?;
    if task_count >= 20 {
        return Err(ApiFailure::invalid("Start a new session after 20 tasks"));
    }
    let control = if let Some(selection) = input.selection {
        let f = s
            .frame
            .as_ref()
            .filter(|f| f.id == selection.frame_id)
            .ok_or_else(|| conflict("The screen changed. Select the control again."))?;
        Some(
            f.controls
                .iter()
                .find(|c| c.id == selection.control_id)
                .cloned()
                .ok_or_else(|| conflict("The selected control is no longer available"))?,
        )
    } else {
        None
    };
    let task = PhoneTask {
        sequence: None,
        steps: vec![],
        generation: None,
        progress: None,
        id: input.id,
        goal: input.goal,
        control,
        state: PhoneTaskState::Queued,
        message: "Waiting for AI".into(),
    };
    phone_tasks::ActiveModel {
        id: Set(task.id),
        session_id: Set(id),
        fingerprint: Set(fingerprint),
        payload: Set(json(&task)?),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    s.state = PhoneState::Acting;
    s.message = "Planning your task".into();
    store(&tx, &s).await?;
    tx.commit().await?;
    detail(&ctx.db, id).await
}
pub async fn stop(ctx: &AppContext, user: Uuid, id: Uuid) -> ApiResult<PhoneSession> {
    authorized(ctx, user, id).await?;
    let tx = ctx.db.begin().await?;
    let r = read(&tx, id, true).await?;
    let worker_id = r.worker_id;
    let mut s: PhoneSession = decode(r.payload)?;
    if !matches!(s.state, PhoneState::Closed | PhoneState::Quarantined) {
        s.state = if worker_id.is_none() {
            PhoneState::Closed
        } else {
            PhoneState::Stopping
        };
        s.message = "Stopping the app session".into();
        store(&tx, &s).await?;
    }
    tx.commit().await?;
    detail(&ctx.db, id).await
}
pub async fn claim(
    ctx: &AppContext,
    w: &Worker,
    input: PhoneClaimRequest,
) -> ApiResult<PhoneClaimResponse> {
    if ![0, 2, 3, 4, 5, 6].contains(&input.protocol_version) {
        return Err(conflict("Unsupported phone protocol"));
    }
    if let Some(capabilities) = input.model_capabilities.as_ref() {
        capabilities
            .validate(input.protocol_version)
            .map_err(ApiFailure::invalid)?;
    }
    reconcile(ctx).await?;
    let tx = ctx.db.begin().await?;
    app_rows::Entity::find_by_id(w.app_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if !super::device_hosts::claim_fence(&tx, w, input.claim_id).await? {
        return Ok(PhoneClaimResponse { lease: None });
    }
    let p = test_definitions::profile(&tx, w.app_id, w.profile_id).await?;
    p.validate().map_err(ApiFailure::invalid)?;
    model_registry::advertise_phone(
        &ctx.db,
        w.id,
        input.protocol_version,
        input.model_capabilities.as_ref(),
    )
    .await?;
    if p.execution_context.is_some() && input.protocol_version < 4 {
        return Ok(PhoneClaimResponse { lease: None });
    }
    if !p.qualified || !matches!(p.driver, Driver::Minitap | Driver::Direct) {
        return Err(conflict("A qualified real device profile is required"));
    }
    if phone_rows::Entity::find()
        .filter(phone_rows::Column::WorkerId.eq(w.id))
        .filter(phone_rows::Column::ClaimId.eq(input.claim_id))
        .one(&tx)
        .await?
        .is_some()
    {
        return Err(conflict(
            "Claim was already delivered; do not replay device work",
        ));
    }
    let resources = [
        format!("app:{}", w.app_id),
        format!("device:{}", p.device_identity),
    ];
    if execution_reservations::Entity::find()
        .filter(execution_reservations::Column::Resource.is_in(resources.clone()))
        .one(&tx)
        .await?
        .is_some()
    {
        return Ok(PhoneClaimResponse { lease: None });
    }
    let pending = phone_rows::Entity::find()
        .filter(phone_rows::Column::AppId.eq(w.app_id))
        .filter(phone_rows::Column::ProfileId.eq(w.profile_id))
        .filter(phone_rows::Column::Deadline.gt(Utc::now()))
        .order_by_asc(phone_rows::Column::CreatedAt)
        .all(&tx)
        .await?;
    let mut selected = None;
    for candidate in pending {
        let session: PhoneSession = decode(candidate.payload.clone())?;
        if session.state != PhoneState::Queued
            || session.resolved_model.as_ref().is_some_and(|resolved| {
                input.protocol_version < 5
                    || !model_registry::worker_matches(resolved, input.model_capabilities.as_ref())
            })
            || session.authoring_model.as_ref().is_some_and(|resolved| {
                input.protocol_version < 5
                    || !model_registry::worker_matches(resolved, input.model_capabilities.as_ref())
            })
        {
            continue;
        }
        if let Some(locked) = phone_rows::Entity::find_by_id(candidate.id)
            .lock_with_behavior(LockType::Update, LockBehavior::SkipLocked)
            .one(&tx)
            .await?
        {
            selected = Some(locked);
            break;
        }
    }
    let Some(row) = selected else {
        return Ok(PhoneClaimResponse { lease: None });
    };
    let mut s: PhoneSession = decode(row.payload.clone())?;
    if let Some(resolved) = s.resolved_model.as_ref() {
        if input.protocol_version < 5
            || !model_registry::worker_matches(resolved, input.model_capabilities.as_ref())
        {
            tracing::warn!(reason_code = "worker_model_reference_mismatch", worker_id=%w.id, "phone claim left queued");
            return Ok(PhoneClaimResponse { lease: None });
        }
    }
    if let Some(resolved) = s.authoring_model.as_ref() {
        if input.protocol_version < 5
            || !model_registry::worker_matches(resolved, input.model_capabilities.as_ref())
        {
            tracing::warn!(reason_code="worker_authoring_model_reference_mismatch",worker_id=%w.id,"phone claim left queued");
            return Ok(PhoneClaimResponse { lease: None });
        }
    }
    super::device_hosts::mark_leased(&tx, w.id).await?;
    s.protocol_version = input.protocol_version;
    if p.driver == Driver::Direct && ![2, 3, 4, 5, 6].contains(&input.protocol_version) {
        return Err(conflict("Update this worker for direct execution"));
    }
    s.state = PhoneState::Preparing;
    s.message = "Opening your app".into();
    // A registered profile cannot change the physical identity of an already queued session.
    if s.profile.device_identity != p.device_identity || s.profile.package != p.package {
        return Err(conflict("Device profile changed; open a new session"));
    }
    let token = loco_rs::hash::random_string(64);
    let now = Utc::now();
    phone_rows::Entity::update_many()
        .col_expr(
            phone_rows::Column::WorkerId,
            sea_orm::sea_query::Expr::value(Some(w.id)),
        )
        .col_expr(
            phone_rows::Column::ClaimId,
            sea_orm::sea_query::Expr::value(Some(input.claim_id)),
        )
        .col_expr(
            phone_rows::Column::LeaseHash,
            sea_orm::sea_query::Expr::value(Some(hash(&token))),
        )
        .col_expr(
            phone_rows::Column::ExpiresAt,
            sea_orm::sea_query::Expr::value(Some(now + Duration::seconds(60))),
        )
        .col_expr(
            phone_rows::Column::Deadline,
            sea_orm::sea_query::Expr::value(now + Duration::minutes(40)),
        )
        .filter(phone_rows::Column::Id.eq(s.id))
        .exec(&tx)
        .await?;
    for resource in resources {
        let inserted =
            execution_reservations::Entity::insert(execution_reservations::ActiveModel {
                resource: Set(resource),
                attempt_id: Set(None),
                session_id: Set(Some(s.id)),
            })
            .on_conflict(
                OnConflict::column(execution_reservations::Column::Resource)
                    .do_nothing()
                    .to_owned(),
            )
            .exec_without_returning(&tx)
            .await?;
        if inserted != 1 {
            return Ok(PhoneClaimResponse { lease: None });
        }
    }
    store(&tx, &s).await?;
    let out = PhoneClaimResponse {
        lease: Some(PhoneLease {
            session: s,
            lease_token: token,
            build_sha256: row.build_sha256,
            build_bytes: u32::try_from(row.build_bytes).map_err(|_| ApiFailure::internal())?,
        }),
    };
    tx.commit().await?;
    Ok(out)
}
pub async fn lease(
    db: &impl ConnectionTrait,
    w: &Worker,
    id: Uuid,
    token: &str,
) -> ApiResult<phone_rows::Model> {
    super::worker_auth::lease_authority(db, w).await?;
    let r = phone_rows::Entity::find_by_id(id)
        .filter(phone_rows::Column::WorkerId.eq(w.id))
        .filter(phone_rows::Column::AppId.eq(w.app_id))
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if token.len() != 64
        || r.lease_hash.as_deref() != Some(hash(token).as_str())
        || r.expires_at.is_none_or(|expires| expires <= Utc::now())
    {
        return Err(conflict("Session lease expired or changed"));
    }
    let s: PhoneSession = decode(r.payload.clone())?;
    if matches!(s.state, PhoneState::Quarantined | PhoneState::Closed) {
        return Err(conflict("Session ended"));
    }
    Ok(r)
}
pub(crate) fn validate_frame(f: &PhoneFrame) -> ApiResult<()> {
    if f.png_base64.len() > 2097152 || f.controls.len() > 200 || f.width != 1080 || f.height != 1920
    {
        return Err(ApiFailure::invalid("Unsupported screen capture"));
    }
    let png = STANDARD
        .decode(&f.png_base64)
        .map_err(|_| ApiFailure::invalid("Invalid screen encoding"))?;
    if png.len() < 24
        || &png[..8] != b"\x89PNG\r\n\x1a\n"
        || png[16..20] != f.width.to_be_bytes()
        || png[20..24] != f.height.to_be_bytes()
    {
        return Err(ApiFailure::invalid("Invalid screen image"));
    }
    let mut ids = std::collections::HashSet::new();
    for c in &f.controls {
        if c.id.len() > 100
            || !ids.insert(&c.id)
            || c.label.len() > 500
            || c.resource_id.len() > 500
            || c.left >= c.right
            || c.top >= c.bottom
            || c.right > f.width
            || c.bottom > f.height
        {
            return Err(ApiFailure::invalid("Invalid screen control"));
        }
    }
    Ok(())
}
pub async fn update(
    ctx: &AppContext,
    w: &Worker,
    id: Uuid,
    token: &str,
    input: PhoneUpdate,
) -> ApiResult<PhoneSession> {
    if input.message.len() > 500 {
        return Err(ApiFailure::invalid("Status message is too long"));
    }
    if let Some(f) = &input.frame {
        validate_frame(f)?;
    }
    let tx = ctx.db.begin().await?;
    let r = lease(&tx, w, id, token).await?;
    let expired = r.deadline < Utc::now();
    let mut s: PhoneSession = decode(r.payload)?;
    if expired {
        s.state = PhoneState::Stopping;
    }
    if input.clean {
        if input.state != PhoneState::Closed {
            return Err(ApiFailure::invalid("Cleanup must close the session"));
        }
        s.state = PhoneState::Closed;
        s.message = "Session closed".into();
        execution_reservations::Entity::delete_many()
            .filter(execution_reservations::Column::SessionId.eq(id))
            .exec(&tx)
            .await?;
        let task_rows = phone_tasks::Entity::find()
            .filter(phone_tasks::Column::SessionId.eq(id))
            .lock_exclusive()
            .all(&tx)
            .await?;
        for row in task_rows {
            let mut task: PhoneTask = decode(row.payload.clone())?;
            if !matches!(task.state, PhoneTaskState::Queued | PhoneTaskState::Acting) {
                continue;
            }
            task.state = PhoneTaskState::Stopped;
            task.message = "Session stopped".into();
            if let Some(progress) = &mut task.progress {
                progress.state = mobile_qa_contracts::automation::GenerationState::Canceled;
                for receipt in &mut progress.journal {
                    if receipt.outcome == mobile_qa_contracts::automation::DiscoveryOutcome::Pending
                    {
                        receipt.outcome =
                            mobile_qa_contracts::automation::DiscoveryOutcome::Uncertain;
                    }
                }
            }
            for step in &mut task.steps {
                if step.state == mobile_qa_contracts::automation::StepState::Started {
                    step.state = mobile_qa_contracts::automation::StepState::Inconclusive;
                    step.message = "Session stopped before the result was acknowledged".into();
                }
            }
            let mut active = row.into_active_model();
            active.payload = Set(json(&task)?);
            active.update(&tx).await?;
        }
    } else {
        if let Some(mut task) = input.task {
            let task_row = phone_tasks::Entity::find_by_id(task.id)
                .filter(phone_tasks::Column::SessionId.eq(id))
                .lock_exclusive()
                .one(&tx)
                .await?
                .ok_or_else(ApiFailure::missing)?;
            let old: PhoneTask = decode(task_row.payload.clone())?;
            if serde_json::to_value(&old).ok() == serde_json::to_value(&task).ok() {
                phone_rows::Entity::update_many()
                    .col_expr(
                        phone_rows::Column::ExpiresAt,
                        sea_orm::sea_query::Expr::value(Some(Utc::now() + Duration::seconds(60))),
                    )
                    .filter(phone_rows::Column::Id.eq(id))
                    .exec(&tx)
                    .await?;
                tx.commit().await?;
                return detail(&ctx.db, id).await;
            }
            if !matches!(old.state, PhoneTaskState::Queued | PhoneTaskState::Acting)
                || task.message.len() > 500
            {
                return Err(conflict("Task already ended"));
            }
            if task.state == PhoneTaskState::Queued {
                return Err(conflict("Task cannot return to the queue"));
            }
            super::authoring_validation::task_update(&old, &task, &s.profile.package)?;
            task.sequence = old.sequence;
            task.generation = old.generation;
            task.goal = old.goal;
            task.control = old.control;
            let ended = matches!(
                task.state,
                PhoneTaskState::Completed | PhoneTaskState::Failed | PhoneTaskState::Stopped
            );
            let mut active = task_row.into_active_model();
            active.payload = Set(json(&task)?);
            active.update(&tx).await?;
            if s.state != PhoneState::Stopping {
                s.state = if ended {
                    PhoneState::Ready
                } else {
                    PhoneState::Acting
                };
            }
        }
        if s.state == PhoneState::Preparing
            && input.state == PhoneState::Ready
            && input.frame.is_some()
        {
            // Preparation has its own bounded allowance; only the first Ready starts interactive time.
            phone_rows::Entity::update_many()
                .col_expr(
                    phone_rows::Column::Deadline,
                    sea_orm::sea_query::Expr::value(Utc::now() + Duration::minutes(20)),
                )
                .filter(phone_rows::Column::Id.eq(id))
                .exec(&tx)
                .await?;
            s.state = PhoneState::Ready;
        }
        if let Some(f) = input.frame {
            s.frame = Some(f);
        }
        s.message = if s.state == PhoneState::Stopping {
            "Stopping the app session".into()
        } else {
            input.message
        };
    }
    store(&tx, &s).await?;
    phone_rows::Entity::update_many()
        .col_expr(
            phone_rows::Column::ExpiresAt,
            sea_orm::sea_query::Expr::value(Some(Utc::now() + Duration::seconds(60))),
        )
        .filter(phone_rows::Column::Id.eq(id))
        .exec(&tx)
        .await?;
    tx.commit().await?;
    detail(&ctx.db, id).await
}
