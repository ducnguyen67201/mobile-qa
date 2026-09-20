//! Durable interactive sessions. A lost lease quarantines the phone instead of replaying effects.
use super::{apps, execution_store::*, test_definitions, worker_auth::Worker};
use crate::errors::{ApiFailure, ApiResult};
use base64::{engine::general_purpose::STANDARD, Engine};
use loco_rs::app::AppContext;
use mobile_qa_contracts::{
    execution::{Driver, ExecutionProfile},
    task_sessions::*,
};
use sea_orm::{ConnectionTrait, QueryResult, TransactionTrait};
use uuid::Uuid;

async fn reconcile(ctx: &AppContext) -> ApiResult<()> {
    exec(&ctx.db,"UPDATE phone_sessions SET payload=jsonb_set(jsonb_set(payload,'{state}','\"quarantined\"'),'{message}','\"Connection lost. The operator must recover this device.\"') WHERE worker_id IS NOT NULL AND expires_at<now() AND payload->>'state' NOT IN ('closed','quarantined')",vec![]).await?;
    exec(&ctx.db,"UPDATE phone_sessions SET payload=jsonb_set(payload,'{state}','\"closed\"') WHERE worker_id IS NULL AND deadline<now()",vec![]).await?;
    Ok(())
}
async fn read(db: &impl ConnectionTrait, id: Uuid, lock: bool) -> ApiResult<QueryResult> {
    one(
        db,
        if lock {
            "SELECT * FROM phone_sessions WHERE id=$1 FOR UPDATE"
        } else {
            "SELECT * FROM phone_sessions WHERE id=$1"
        },
        vec![id.into()],
    )
    .await
}
async fn store(db: &impl ConnectionTrait, s: &PhoneSession) -> ApiResult<()> {
    let mut snapshot = s.clone();
    snapshot.tasks.clear();
    exec(
        db,
        "UPDATE phone_sessions SET payload=$2 WHERE id=$1",
        vec![s.id.into(), json(&snapshot)?.into()],
    )
    .await?;
    Ok(())
}
pub async fn detail(db: &impl ConnectionTrait, id: Uuid) -> ApiResult<PhoneSession> {
    let mut s: PhoneSession = decode(field(&read(db, id, false).await?, "payload")?)?;
    s.tasks = rows(
        db,
        "SELECT payload FROM phone_tasks WHERE session_id=$1 ORDER BY created_at,id",
        vec![id.into()],
    )
    .await?
    .iter()
    .map(|r| decode(field(r, "payload")?))
    .collect::<ApiResult<_>>()?;
    Ok(s)
}
pub async fn authorized(ctx: &AppContext, user: Uuid, id: Uuid) -> ApiResult<PhoneSession> {
    reconcile(ctx).await?;
    let r = read(&ctx.db, id, false).await?;
    apps::authorized(ctx, user, field(&r, "app_id")?).await?;
    // Only the session creator controls or views the disposable account state.
    if field::<Uuid>(&r, "creator_id")? != user {
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
    let builds=rows(&ctx.db,"SELECT id,original_filename FROM builds WHERE app_id=$1 AND validation_state='validated' ORDER BY created_at DESC,id DESC LIMIT 50",vec![app.into()]).await?.iter().map(|r|Ok(PhoneBuildChoice{id:field(r,"id")?,name:field(r,"original_filename")?})).collect::<ApiResult<Vec<_>>>()?;
    let profiles=rows(&ctx.db,"SELECT p.payload FROM execution_profiles p WHERE p.app_id=$1 AND EXISTS(SELECT 1 FROM execution_workers w WHERE w.profile_id=p.id AND w.revoked=false)",vec![app.into()]).await?.iter().map(|r|decode::<ExecutionProfile>(field(r,"payload")?)).collect::<ApiResult<Vec<_>>>()?.into_iter().filter(|p|p.qualified&&p.validate().is_ok()&&matches!(p.driver,Driver::Minitap|Driver::Direct)&&p.package==a.android_package).collect::<Vec<_>>();
    let active_session=rows(&ctx.db,"SELECT id FROM phone_sessions WHERE app_id=$1 AND creator_id=$2 AND payload->>'state' NOT IN ('closed','quarantined') ORDER BY created_at DESC LIMIT 1",vec![app.into(),user.into()]).await?.first().map(|r|field(r,"id")).transpose()?;
    let mut blockers = vec![];
    let env = one(&ctx.db,"SELECT account_secret_reference_id,reset_secret_reference_id FROM environments WHERE app_id=$1",vec![app.into()]).await?;
    if field::<Option<Uuid>>(&env, "account_secret_reference_id")?.is_some()
        || field::<Option<Uuid>>(&env, "reset_secret_reference_id")?.is_some()
    {
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
    let choices = options(ctx, user, app).await?;
    let fingerprint = hash(serde_json::to_vec(&input).map_err(|_| ApiFailure::internal())?);
    let tx = ctx.db.begin().await?;
    one(
        &tx,
        "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
        vec![app.into()],
    )
    .await?;
    if let Some(r) = rows(
        &tx,
        "SELECT * FROM phone_sessions WHERE id=$1",
        vec![input.id.into()],
    )
    .await?
    .first()
    {
        if field::<Uuid>(r, "app_id")? != app
            || field::<Uuid>(r, "creator_id")? != user
            || field::<String>(r, "fingerprint")? != fingerprint
        {
            return Err(conflict("Session request identity was already used"));
        }
        return detail(&tx, input.id).await;
    }
    let active=rows(&tx,"SELECT id FROM phone_sessions WHERE app_id=$1 AND creator_id=$2 AND payload->>'state' NOT IN ('closed','quarantined')",vec![app.into(),user.into()]).await?;
    if let Some(r) = active.first() {
        return detail(&tx, field(r, "id")?).await;
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
    let b = one(
        &tx,
        "SELECT * FROM builds WHERE id=$1 AND app_id=$2 AND validation_state='validated'",
        vec![build.into(), app.into()],
    )
    .await?;
    let bytes: i64 = field(&b, "byte_size")?;
    if bytes < 1 || bytes > i64::from(profile.max_apk_bytes.min(104857600)) {
        return Err(ApiFailure::invalid("Build is too large for this device"));
    }
    let environment_revision: i32 = field(
        &one(
            &tx,
            "SELECT revision FROM environments WHERE app_id=$1",
            vec![app.into()],
        )
        .await?,
        "revision",
    )?;
    let s = PhoneSession {
        environment_revision: environment_revision as u32,
        revision: 0,
        protocol_version: 0,
        id: input.id,
        app_id: app,
        build_id: build,
        profile: profile.clone(),
        state: PhoneState::Queued,
        message: "Waiting for a device".into(),
        frame: None,
        tasks: vec![],
    };
    exec(&tx,"INSERT INTO phone_sessions(id,app_id,creator_id,build_id,profile_id,payload,fingerprint,build_sha256,build_bytes) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)",vec![s.id.into(),app.into(),user.into(),build.into(),profile.id.into(),json(&s)?.into(),fingerprint.into(),field::<String>(&b,"sha256")?.into(),(bytes as i32).into()]).await?;
    tx.commit().await?;
    Ok(s)
}
pub async fn task(
    ctx: &AppContext,
    user: Uuid,
    id: Uuid,
    input: PhoneTaskRequest,
) -> ApiResult<PhoneSession> {
    authorized(ctx, user, id).await?;
    if input.goal.trim().is_empty() || input.goal.len() > 4000 {
        return Err(ApiFailure::invalid(
            "Describe a task using 1–4000 characters",
        ));
    }
    let tx = ctx.db.begin().await?;
    let r = read(&tx, id, true).await?;
    let mut s: PhoneSession = decode(field(&r, "payload")?)?;
    let fingerprint = hash(serde_json::to_vec(&input).map_err(|_| ApiFailure::internal())?);
    if let Some(prior) = rows(
        &tx,
        "SELECT session_id,fingerprint FROM phone_tasks WHERE id=$1",
        vec![input.id.into()],
    )
    .await?
    .first()
    {
        if field::<Uuid>(prior, "session_id")? != id
            || field::<String>(prior, "fingerprint")? != fingerprint
        {
            return Err(conflict("Task request identity was already used"));
        }
        return detail(&tx, id).await;
    }
    if s.profile.model.is_empty() {
        return Err(ApiFailure::invalid(
            "Legacy tasks require a model; use structured commands for direct execution",
        ));
    }
    if s.state != PhoneState::Ready {
        return Err(conflict("Wait for the phone to be ready"));
    }
    let tasks = rows(
        &tx,
        "SELECT id FROM phone_tasks WHERE session_id=$1",
        vec![id.into()],
    )
    .await?;
    if tasks.len() >= 20 {
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
    exec(
        &tx,
        "INSERT INTO phone_tasks(id,session_id,fingerprint,payload) VALUES($1,$2,$3,$4)",
        vec![
            task.id.into(),
            id.into(),
            fingerprint.into(),
            json(&task)?.into(),
        ],
    )
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
    let mut s: PhoneSession = decode(field(&r, "payload")?)?;
    if !matches!(s.state, PhoneState::Closed | PhoneState::Quarantined) {
        s.state = if field::<Option<Uuid>>(&r, "worker_id")?.is_none() {
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
    if ![0, 2, 3, 4].contains(&input.protocol_version) {
        return Err(conflict("Unsupported phone protocol"));
    }
    reconcile(ctx).await?;
    let tx = ctx.db.begin().await?;
    one(
        &tx,
        "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
        vec![w.app_id.into()],
    )
    .await?;
    let p = test_definitions::profile(&tx, w.app_id, w.profile_id).await?;
    p.validate().map_err(ApiFailure::invalid)?;
    if p.execution_context.is_some() && input.protocol_version < 4 {
        return Ok(PhoneClaimResponse { lease: None });
    }
    if !p.qualified || !matches!(p.driver, Driver::Minitap | Driver::Direct) {
        return Err(conflict("A qualified real device profile is required"));
    }
    rows(
        &tx,
        "SELECT pg_advisory_xact_lock(hashtextextended($1,0))",
        vec![p.device_identity.clone().into()],
    )
    .await?;
    if !rows(
        &tx,
        "SELECT id FROM phone_sessions WHERE worker_id=$1 AND claim_id=$2",
        vec![w.id.into(), input.claim_id.into()],
    )
    .await?
    .is_empty()
    {
        return Err(conflict(
            "Claim was already delivered; do not replay device work",
        ));
    }
    if !rows(
        &tx,
        "SELECT resource FROM execution_reservations WHERE resource=$1 OR resource=$2",
        vec![
            format!("app:{}", w.app_id).into(),
            format!("device:{}", p.device_identity).into(),
        ],
    )
    .await?
    .is_empty()
    {
        return Ok(PhoneClaimResponse { lease: None });
    }
    let pending=rows(&tx,"SELECT * FROM phone_sessions WHERE app_id=$1 AND profile_id=$2 AND payload->>'state'='queued' AND deadline>now() ORDER BY created_at LIMIT 1 FOR UPDATE SKIP LOCKED",vec![w.app_id.into(),w.profile_id.into()]).await?;
    let Some(r) = pending.first() else {
        return Ok(PhoneClaimResponse { lease: None });
    };
    let mut s: PhoneSession = decode(field(r, "payload")?)?;
    s.protocol_version = input.protocol_version;
    if p.driver == Driver::Direct && ![2, 3, 4].contains(&input.protocol_version) {
        return Err(conflict("Update this worker for direct execution"));
    }
    s.state = PhoneState::Preparing;
    s.message = "Opening your app".into();
    // A registered profile cannot change the physical identity of an already queued session.
    if s.profile.device_identity != p.device_identity || s.profile.package != p.package {
        return Err(conflict("Device profile changed; open a new session"));
    }
    let token = loco_rs::hash::random_string(64);
    exec(&tx,"UPDATE phone_sessions SET worker_id=$2,claim_id=$3,lease_hash=$4,expires_at=now()+interval '60 seconds' WHERE id=$1",vec![s.id.into(),w.id.into(),input.claim_id.into(),hash(&token).into()]).await?;
    for resource in [
        format!("app:{}", w.app_id),
        format!("device:{}", p.device_identity),
    ] {
        exec(
            &tx,
            "INSERT INTO execution_reservations(resource,session_id) VALUES($1,$2)",
            vec![resource.into(), s.id.into()],
        )
        .await?;
    }
    store(&tx, &s).await?;
    let out = PhoneClaimResponse {
        lease: Some(PhoneLease {
            session: s,
            lease_token: token,
            build_sha256: field(r, "build_sha256")?,
            build_bytes: field::<i32>(r, "build_bytes")? as u32,
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
) -> ApiResult<QueryResult> {
    let r=one(db,"SELECT *, expires_at>now() AS alive FROM phone_sessions WHERE id=$1 AND worker_id=$2 AND app_id=$3 FOR UPDATE",vec![id.into(),w.id.into(),w.app_id.into()]).await?;
    if token.len() != 64
        || field::<Option<String>>(&r, "lease_hash")? != Some(hash(token))
        || !field::<bool>(&r, "alive")?
    {
        return Err(conflict("Session lease expired or changed"));
    }
    let s: PhoneSession = decode(field(&r, "payload")?)?;
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
    let mut s: PhoneSession = decode(field(&r, "payload")?)?;
    let expired: bool = field(
        &one(
            &tx,
            "SELECT deadline<now() AS expired FROM phone_sessions WHERE id=$1",
            vec![id.into()],
        )
        .await?,
        "expired",
    )?;
    if expired {
        s.state = PhoneState::Stopping;
    }
    if input.clean {
        if input.state != PhoneState::Closed {
            return Err(ApiFailure::invalid("Cleanup must close the session"));
        }
        s.state = PhoneState::Closed;
        s.message = "Session closed".into();
        exec(
            &tx,
            "DELETE FROM execution_reservations WHERE session_id=$1",
            vec![id.into()],
        )
        .await?;
        for row in rows(&tx,"SELECT id,payload FROM phone_tasks WHERE session_id=$1 AND payload->>'state' IN ('queued','acting') FOR UPDATE",vec![id.into()]).await? {
            let mut task:PhoneTask=decode(field(&row,"payload")?)?;
            task.state=PhoneTaskState::Stopped;
            task.message="Session stopped".into();
            if let Some(progress)=&mut task.progress {
                progress.state=mobile_qa_contracts::automation::GenerationState::Canceled;
                for receipt in &mut progress.journal {
                    if receipt.outcome==mobile_qa_contracts::automation::DiscoveryOutcome::Pending {
                        receipt.outcome=mobile_qa_contracts::automation::DiscoveryOutcome::Uncertain;
                    }
                }
            }
            for step in &mut task.steps {if step.state==mobile_qa_contracts::automation::StepState::Started {step.state=mobile_qa_contracts::automation::StepState::Inconclusive;step.message="Session stopped before the result was acknowledged".into();}}
            exec(&tx,"UPDATE phone_tasks SET payload=$2 WHERE id=$1",vec![task.id.into(),json(&task)?.into()]).await?;
        }
    } else {
        if let Some(mut task) = input.task {
            let old: PhoneTask = decode(field(
                &one(
                    &tx,
                    "SELECT payload FROM phone_tasks WHERE id=$1 AND session_id=$2 FOR UPDATE",
                    vec![task.id.into(), id.into()],
                )
                .await?,
                "payload",
            )?)?;
            if serde_json::to_value(&old).ok() == serde_json::to_value(&task).ok() {
                exec(
                    &tx,
                    "UPDATE phone_sessions SET expires_at=now()+interval '60 seconds' WHERE id=$1",
                    vec![id.into()],
                )
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
            exec(
                &tx,
                "UPDATE phone_tasks SET payload=$2 WHERE id=$1",
                vec![task.id.into(), json(&task)?.into()],
            )
            .await?;
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
    exec(
        &tx,
        "UPDATE phone_sessions SET expires_at=now()+interval '60 seconds' WHERE id=$1",
        vec![id.into()],
    )
    .await?;
    tx.commit().await?;
    detail(&ctx.db, id).await
}
