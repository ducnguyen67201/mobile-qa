//! Short PostgreSQL leases plus persistent reservations; never replay device effects on expiry.
use super::{execution_store::*, runs, test_definitions, worker_auth::Worker};
use crate::errors::{ApiFailure, ApiResult};
use chrono::{DateTime, Utc};
use loco_rs::app::AppContext;
use mobile_qa_contracts::execution::*;
use sea_orm::{ConnectionTrait, QueryResult, TransactionTrait};
use uuid::Uuid;

pub async fn reconcile(ctx: &AppContext) -> ApiResult<()> {
    exec(
        &ctx.db,
        "UPDATE execution_attempts SET state='recovery_required',generation=generation+1,\
            cleanup='quarantined',outcome=COALESCE(outcome,'inconclusive'),\
            reason='lease_expired' WHERE state IN ('leased','running','finalizing',\
            'cancel_requested') AND expires_at<now()",
        vec![],
    )
    .await?;
    Ok(())
}

pub async fn lease(
    db: &impl ConnectionTrait,
    worker: &Worker,
    id: Uuid,
    generation: i32,
    token: &str,
    lock: bool,
) -> ApiResult<QueryResult> {
    let sql = if lock {
        "SELECT a.*,r.app_id,r.manifest,r.cancel_requested FROM execution_attempts a JOIN \
            execution_runs r ON a.run_id=r.id WHERE a.id=$1 AND a.worker_id=$2 AND r.app_id=$3 \
            FOR UPDATE OF a"
    } else {
        "SELECT a.*,r.app_id,r.manifest,r.cancel_requested FROM execution_attempts a JOIN \
            execution_runs r ON a.run_id=r.id WHERE a.id=$1 AND a.worker_id=$2 AND r.app_id=$3"
    };
    let r = one(
        db,
        sql,
        vec![id.into(), worker.id.into(), worker.app_id.into()],
    )
    .await?;
    if token.len() < 32
        || field::<Option<String>>(&r, "lease_hash")?.as_deref() != Some(hash(token).as_str())
        || field::<i32>(&r, "generation")? != generation
    {
        return Err(conflict("Lease authority is stale"));
    }
    let state: String = field(&r, "state")?;
    let alive: bool = field(
        &one(
            db,
            "SELECT ($1::timestamptz>now()) AS alive",
            vec![field::<Option<DateTime<Utc>>>(&r, "expires_at")?.into()],
        )
        .await?,
        "alive",
    )?;
    if state == "recovery_required" || (!alive && state != "finished") {
        return Err(conflict("Lease expired; resource requires recovery"));
    }
    Ok(r)
}
fn token() -> String {
    loco_rs::hash::random_string(64)
}

/// Wait without holding a transaction or device reservation. Subscribe before the
/// first query so a commit between checking the queue and waiting cannot be lost.
pub async fn claim_wait(
    ctx: &AppContext,
    worker: &Worker,
    input: ClaimRequest,
    wait: std::time::Duration,
) -> ApiResult<ClaimResponse> {
    use tokio::time::{Duration, Instant};
    let mut wakeup = super::execution_wakeup::subscribe(ctx);
    let deadline = Instant::now() + wait.min(Duration::from_secs(30));
    loop {
        // Authentication may have been revoked while this HTTP request was asleep.
        let registered = rows(
            &ctx.db,
            "SELECT id FROM execution_workers WHERE id=$1 AND app_id=$2 \
                AND profile_id=$3 AND revoked=false",
            vec![
                worker.id.into(),
                worker.app_id.into(),
                worker.profile_id.into(),
            ],
        )
        .await?;
        if registered.is_empty() {
            return Err(ApiFailure::unauthorized());
        }
        let mut response = claim(ctx, worker, input.clone()).await?;
        if response.lease.is_some() || Instant::now() >= deadline {
            response.poll_after_seconds = 0;
            return Ok(response);
        }
        // Local commits wake immediately. Other API processes are discovered within
        // five seconds; PostgreSQL remains authoritative if a notification is lost.
        let until = deadline.min(Instant::now() + Duration::from_secs(5));
        tokio::select! {
            _ = wakeup.changed() => {},
            _ = tokio::time::sleep_until(until) => {},
        }
    }
}

pub async fn claim(
    ctx: &AppContext,
    worker: &Worker,
    input: ClaimRequest,
) -> ApiResult<ClaimResponse> {
    if input.version != 1 || input.profile_id != worker.profile_id {
        return Err(conflict("Unsupported protocol or worker profile"));
    }
    reconcile(ctx).await?;
    let tx = ctx.db.begin().await?;
    one(
        &tx,
        "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
        vec![worker.app_id.into()],
    )
    .await?;
    let profile = test_definitions::profile(&tx, worker.app_id, worker.profile_id).await?;
    // The advisory lock serializes shared physical identity even across app-scoped workers.
    rows(
        &tx,
        "SELECT pg_advisory_xact_lock(hashtextextended($1,0))",
        vec![profile.device_identity.clone().into()],
    )
    .await?;
    let prior = rows(
        &tx,
        "SELECT id,state FROM execution_attempts WHERE worker_id=$1 AND claim_id=$2",
        vec![worker.id.into(), input.claim_id.into()],
    )
    .await?;
    let id = if let Some(r) = prior.first() {
        let state: String = field(r, "state")?;
        if !["leased", "running", "finalizing", "cancel_requested"].contains(&state.as_str()) {
            return Err(conflict("Claim already ended; reconcile local journal"));
        }
        field(r, "id")?
    } else {
        if !profile.qualified {
            return Err(ApiFailure::invalid("Worker profile is not qualified"));
        }
        let busy = rows(
            &tx,
            "SELECT resource FROM execution_reservations WHERE resource=$1 OR resource=$2",
            vec![
                format!("app:{}", worker.app_id).into(),
                format!("device:{}", profile.device_identity).into(),
            ],
        )
        .await?;
        if !busy.is_empty() {
            return Ok(ClaimResponse {
                lease: None,
                poll_after_seconds: 5,
            });
        }
        let jobs = rows(
            &tx,
            "SELECT a.id FROM execution_attempts a JOIN execution_runs r ON r.id=a.run_id WHERE \
            r.app_id=$1 AND a.state='queued' AND r.cancel_requested=false AND \
            r.manifest->'profile'->>'id'=$2 ORDER BY r.created_at,a.case_index,a.number FOR \
            UPDATE OF a SKIP LOCKED LIMIT 1",
            vec![worker.app_id.into(), worker.profile_id.to_string().into()],
        )
        .await?;
        let Some(r) = jobs.first() else {
            return Ok(ClaimResponse {
                lease: None,
                poll_after_seconds: 5,
            });
        };
        let id: Uuid = field(r, "id")?;
        for resource in [
            format!("app:{}", worker.app_id),
            format!("device:{}", profile.device_identity),
        ] {
            exec(
                &tx,
                "INSERT INTO execution_reservations(resource,attempt_id) VALUES($1,$2)",
                vec![resource.into(), id.into()],
            )
            .await?;
        }
        exec(
            &tx,
            "UPDATE execution_attempts SET worker_id=$2,claim_id=$3,state='leased' WHERE id=$1",
            vec![id.into(), worker.id.into(), input.claim_id.into()],
        )
        .await?;
        id
    };
    let token = token();
    let r = one(
        &tx,
        "UPDATE execution_attempts SET lease_hash=$2,expires_at=now()+interval '60 seconds' \
            WHERE id=$1 RETURNING *",
        vec![id.into(), hash(&token).into()],
    )
    .await?;
    let run_id: Uuid = field(&r, "run_id")?;
    let manifest = decode(field(
        &one(
            &tx,
            "SELECT manifest FROM execution_runs WHERE id=$1",
            vec![run_id.into()],
        )
        .await?,
        "manifest",
    )?)?;
    let result = ClaimResponse {
        lease: Some(ExecutionLease {
            attempt_id: id,
            run_id,
            generation: field(&r, "generation")?,
            lease_token: token,
            expires_at: field(&r, "expires_at")?,
            manifest,
            case_index: field::<i32>(&r, "case_index")? as u32,
        }),
        poll_after_seconds: 5,
    };
    tx.commit().await?;
    Ok(result)
}

pub async fn heartbeat(
    ctx: &AppContext,
    w: &Worker,
    id: Uuid,
    g: i32,
    token: &str,
) -> ApiResult<LeaseStatusResponse> {
    let tx = ctx.db.begin().await?;
    let r = lease(&tx, w, id, g, token, true).await?;
    let cancel: bool = field(&r, "cancel_requested")?;
    let r=one(&tx,"UPDATE execution_attempts SET expires_at=now()+interval '60 seconds',state=CASE \
            WHEN state='leased' THEN 'running' ELSE state END WHERE id=$1 RETURNING state,expires_at",vec![id.into()]).await?;
    let out = LeaseStatusResponse {
        state: decode(serde_json::Value::String(field(&r, "state")?))?,
        expires_at: field(&r, "expires_at")?,
        cancel_requested: cancel,
    };
    tx.commit().await?;
    Ok(out)
}

pub async fn events(
    ctx: &AppContext,
    w: &Worker,
    id: Uuid,
    token: &str,
    input: EventRequest,
) -> ApiResult<EventReceipt> {
    if input.events.is_empty() || input.events.len() > 50 {
        return Err(ApiFailure::invalid("Send 1–50 events"));
    }
    let tx = ctx.db.begin().await?;
    let r = lease(&tx, w, id, input.generation, token, true).await?;
    let manifest: RunManifest = decode(field(&r, "manifest")?)?;
    let case = &manifest.cases[field::<i32>(&r, "case_index")? as usize].case;
    let mut last: i64 = field(
        &one(
            &tx,
            "SELECT COALESCE(max(sequence),0)::bigint AS last FROM \
            execution_events WHERE attempt_id=$1",
            vec![id.into()],
        )
        .await?,
        "last",
    )?;
    for event in &input.events {
        if event.sequence == 0
            || event.sequence > 10000
            || !bounded(&event.phase, 80)
            || event.message.len() > 1000
            || !case.actions.iter().any(|a| a.id == event.action_id)
        {
            return Err(ApiFailure::invalid("Invalid event"));
        }
        let payload = json(event)?;
        let old = rows(
            &tx,
            "SELECT attempt_id,payload FROM execution_events WHERE id=$1 OR (attempt_id=$2 AND \
            sequence=$3)",
            vec![event.id.into(), id.into(), (event.sequence as i32).into()],
        )
        .await?;
        if !old.is_empty() {
            if old.len() != 1
                || field::<Uuid>(&old[0], "attempt_id")? != id
                || field::<serde_json::Value>(&old[0], "payload")? != payload
            {
                return Err(conflict("Event identity conflicts"));
            }
            continue;
        }
        if !["leased", "running", "cancel_requested"]
            .contains(&field::<String>(&r, "state")?.as_str())
            || i64::from(event.sequence) != last + 1
        {
            return Err(conflict(
                "Event sequence or attempt state does not accept new events",
            ));
        }
        exec(
            &tx,
            "INSERT INTO execution_events(id,attempt_id,sequence,payload) VALUES($1,$2,$3,$4)",
            vec![
                event.id.into(),
                id.into(),
                (event.sequence as i32).into(),
                payload.into(),
            ],
        )
        .await?;
        last += 1;
    }
    tx.commit().await?;
    Ok(EventReceipt {
        last_sequence: last as u32,
    })
}

pub async fn complete(
    ctx: &AppContext,
    w: &Worker,
    id: Uuid,
    token: &str,
    input: CompleteRequest,
) -> ApiResult<AttemptReceipt> {
    if !bounded(&input.reason, 200)
        || input.usage.len() > 20
        || input
            .usage
            .iter()
            .any(|u| !bounded(&u.model, 100) || u.calls > 10000 || u.unknown_calls > u.calls)
    {
        return Err(ApiFailure::invalid("Invalid completion metadata"));
    }
    let digest = hash(serde_json::to_vec(&input).map_err(|_| ApiFailure::internal())?);
    let row = lease(&ctx.db, w, id, input.generation, token, false).await?;
    let manifest: RunManifest = decode(field(&row, "manifest")?)?;
    let case = &manifest.cases[field::<i32>(&row, "case_index")? as usize].case;
    // Capture/evaluation performs bounded storage I/O without a database lock.
    let (checks, mut outcome) = super::verification::evaluate(ctx, id, case).await?;
    if outcome != Outcome::Failed && input.execution_outcome != Outcome::Passed {
        outcome = match input.execution_outcome {
            Outcome::Blocked => Outcome::Blocked,
            Outcome::Canceled => Outcome::Canceled,
            _ => Outcome::Inconclusive,
        };
    }
    let tx = ctx.db.begin().await?;
    // Match cancellation's lock order: run before attempt.
    one(
        &tx,
        "SELECT id FROM execution_runs WHERE id=$1 FOR UPDATE",
        vec![field::<Uuid>(&row, "run_id")?.into()],
    )
    .await?;
    let r = lease(&tx, w, id, input.generation, token, true).await?;
    if let Some(old) = field::<Option<String>>(&r, "completion_hash")? {
        if old != digest {
            return Err(conflict("Completion payload changed"));
        }
        let result = runs::attempt(&tx, id).await?;
        tx.commit().await?;
        return Ok(AttemptReceipt { attempt: result });
    }
    if field::<bool>(&r, "cancel_requested")? && outcome != Outcome::Failed {
        outcome = Outcome::Canceled;
    }
    exec(
        &tx,
        "UPDATE execution_attempts SET state='finalizing',outcome=$2,checks=$3,usage=$4,\
            reason=$5,completion_hash=$6 WHERE id=$1",
        vec![
            id.into(),
            word(&outcome).into(),
            json(&checks)?.into(),
            json(&input.usage)?.into(),
            input.reason.into(),
            digest.into(),
        ],
    )
    .await?;
    let result = runs::attempt(&tx, id).await?;
    tx.commit().await?;
    Ok(AttemptReceipt { attempt: result })
}

pub async fn cleanup(
    ctx: &AppContext,
    w: &Worker,
    id: Uuid,
    token: &str,
    input: CleanupRequest,
) -> ApiResult<AttemptReceipt> {
    if !bounded(&input.evidence_reference, 1000)
        || !bounded(&input.boot_id, 200)
        || input.reset == CleanupState::Pending
    {
        return Err(ApiFailure::invalid(
            "Cleanup evidence and final reset state are required",
        ));
    }
    let digest = hash(serde_json::to_vec(&input).map_err(|_| ApiFailure::internal())?);
    let tx = ctx.db.begin().await?;
    let preliminary = lease(&tx, w, id, input.generation, token, false).await?;
    let run: Uuid = field(&preliminary, "run_id")?;
    one(
        &tx,
        "SELECT id FROM execution_runs WHERE id=$1 FOR UPDATE",
        vec![run.into()],
    )
    .await?;
    let r = lease(&tx, w, id, input.generation, token, true).await?;
    if let Some(old) = field::<Option<String>>(&r, "cleanup_hash")? {
        if old != digest {
            return Err(conflict("Cleanup payload changed"));
        }
        let a = runs::attempt(&tx, id).await?;
        tx.commit().await?;
        return Ok(AttemptReceipt { attempt: a });
    }
    if field::<Option<String>>(&r, "completion_hash")?.is_none() {
        return Err(conflict(
            "Persist attempt evidence before acknowledging cleanup",
        ));
    }
    let clean = input.stopped && input.reset == CleanupState::VerifiedClean;
    exec(
        &tx,
        "UPDATE execution_attempts SET state=$2,cleanup=$3,cleanup_hash=$4,\
            cleanup_receipt=$5 WHERE id=$1",
        vec![
            id.into(),
            if clean {
                "finished"
            } else {
                "recovery_required"
            }
            .into(),
            if clean {
                "verified_clean"
            } else {
                "quarantined"
            }
            .into(),
            digest.into(),
            json(&input)?.into(),
        ],
    )
    .await?;
    if clean {
        exec(
            &tx,
            "DELETE FROM execution_reservations WHERE attempt_id=$1",
            vec![id.into()],
        )
        .await?;
        let manifest: RunManifest = decode(field(&r, "manifest")?)?;
        if manifest.diagnostic_retries == 1
            && field::<i32>(&r, "number")? == 1
            && !field::<bool>(&r, "cancel_requested")?
            && matches!(
                field::<Option<String>>(&r, "outcome")?.as_deref(),
                Some("failed" | "inconclusive")
            )
        {
            exec(
                &tx,
                "INSERT INTO execution_attempts(id,run_id,case_index,number) VALUES($1,$2,$3,2)",
                vec![
                    Uuid::new_v4().into(),
                    run.into(),
                    field::<i32>(&r, "case_index")?.into(),
                ],
            )
            .await?;
        }
    }
    let result = runs::attempt(&tx, id).await?;
    tx.commit().await?;
    super::execution_wakeup::notify(ctx);
    Ok(AttemptReceipt { attempt: result })
}
/// Trusted operator reconciles physical recovery; never changes the original outcome.
pub async fn recover(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    id: Uuid,
    evidence: &str,
) -> ApiResult<()> {
    test_definitions::operator(ctx, actor, app).await?;
    if !bounded(evidence, 1000) {
        return Err(ApiFailure::invalid("Recovery evidence reference required"));
    }
    let tx = ctx.db.begin().await?;
    let r = one(
        &tx,
        "SELECT a.id FROM execution_attempts a JOIN execution_runs r ON r.id=a.run_id WHERE \
            a.id=$1 AND r.app_id=$2 AND a.state='recovery_required' FOR UPDATE OF a",
        vec![id.into(), app.into()],
    )
    .await?;
    let _: Uuid = field(&r, "id")?;
    exec(
        &tx,
        "UPDATE execution_attempts SET state='finished',cleanup='verified_clean',\
            cleanup_receipt=$2 WHERE id=$1",
        vec![
            id.into(),
            serde_json::json!({"operator_id":actor,"recovery_evidence":evidence}).into(),
        ],
    )
    .await?;
    exec(
        &tx,
        "DELETE FROM execution_reservations WHERE attempt_id=$1",
        vec![id.into()],
    )
    .await?;
    tx.commit().await?;
    super::execution_wakeup::notify(ctx);
    Ok(())
}
