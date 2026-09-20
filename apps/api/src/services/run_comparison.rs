//! Baseline selection and comparison are projections over immutable run evidence.
use super::{execution_store::*, runs};
use crate::errors::{ApiFailure, ApiResult};
use chrono::{DateTime, Utc};
use mobile_qa_contracts::execution::*;
use sea_orm::ConnectionTrait;
use std::collections::BTreeMap;
use uuid::Uuid;

struct CleanResult {
    outcome: Outcome,
    checks: Vec<CheckResult>,
}

fn comparison_label(baseline: Outcome, current: Outcome, same_build: bool) -> ComparisonLabel {
    match (baseline, current, same_build) {
        (Outcome::Passed, Outcome::Failed, true) => ComparisonLabel::NewFailureSameBuild,
        (Outcome::Passed, Outcome::Failed, false) => ComparisonLabel::Regression,
        (Outcome::Failed, Outcome::Failed, _) => ComparisonLabel::StillFailing,
        (Outcome::Failed, Outcome::Passed, _) => ComparisonLabel::Recovered,
        (Outcome::Passed, Outcome::Passed, _) => ComparisonLabel::Unchanged,
        _ => ComparisonLabel::NotComparable,
    }
}

fn context_mismatch(baseline: &RunManifest, current: &RunManifest) -> Option<&'static str> {
    if baseline.app_id != current.app_id {
        return Some("Baseline belongs to another app");
    }
    if baseline.environment_revision != current.environment_revision {
        return Some("Environment revision changed");
    }
    if baseline.profile.id != current.profile.id
        || baseline.profile.driver != current.profile.driver
        || baseline.profile.package != current.profile.package
        || baseline.profile.adapter != current.profile.adapter
        || baseline.profile.device_identity != current.profile.device_identity
        || baseline.profile.image != current.profile.image
        || baseline.profile.model != current.profile.model
        || baseline.profile.qualified != current.profile.qualified
        || baseline.profile.qualification_reference != current.profile.qualification_reference
        || baseline.profile.execution_context != current.profile.execution_context
    {
        return Some("Qualified execution context changed");
    }
    match (&baseline.source, &current.source) {
        (
            Some(ManifestSource::SavedCase {
                version: 1,
                case_version_id: baseline_id,
                content_hash: baseline_hash,
            }),
            Some(ManifestSource::SavedCase {
                version: 1,
                case_version_id: current_id,
                content_hash: current_hash,
            }),
        ) if baseline_id == current_id && baseline_hash == current_hash => {}
        (
            Some(ManifestSource::ReleasePlan {
                version: 1,
                plan_version_id: baseline_id,
                content_hash: baseline_hash,
            }),
            Some(ManifestSource::ReleasePlan {
                version: 1,
                plan_version_id: current_id,
                content_hash: current_hash,
            }),
        ) if baseline_id == current_id && baseline_hash == current_hash => {}
        _ => return Some("Saved source version or content changed"),
    }
    if baseline.cases.len() != current.cases.len() {
        return Some("Case coverage changed");
    }
    for current_case in &current.cases {
        let Some(baseline_case) = baseline
            .cases
            .iter()
            .find(|candidate| candidate.case.key == current_case.case.key)
        else {
            return Some("Case identity changed");
        };
        if baseline_case.definition_id != current_case.definition_id
            || baseline_case.content_hash != current_case.content_hash
            || baseline_case.data_variant != current_case.data_variant
        {
            return Some("Saved case version, content or data changed");
        }
    }
    None
}

fn clean_result(run: &RunResponse) -> Result<CleanResult, &'static str> {
    if run.state != JobState::Finished {
        return Err("Run has not finished");
    }
    if run.attempts.is_empty() {
        return Err("Run has no retained attempts");
    }
    let mut outcome = None;
    let mut checks: Option<&[CheckResult]> = None;
    for attempt in &run.attempts {
        if attempt.state != JobState::Finished
            || attempt.cleanup != CleanupState::VerifiedClean
            || !attempt.recovery_events.is_empty()
            || attempt.original_cleanup.as_ref().is_some_and(|receipt| {
                !receipt.stopped || receipt.reset != CleanupState::VerifiedClean
            })
            || (run.manifest.profile.execution_context.is_some() && attempt.preflight.is_none())
        {
            return Err("Start or cleanup proof is incomplete");
        }
        let attempt_outcome = attempt.outcome.ok_or("Attempt outcome is missing")?;
        if !matches!(attempt_outcome, Outcome::Passed | Outcome::Failed) {
            return Err("Blocked, canceled, skipped or inconclusive attempts cannot be compared");
        }
        if outcome.is_some_and(|previous| previous != attempt_outcome) {
            return Err("Retry outcomes disagree");
        }
        if checks.is_some_and(|previous| previous != attempt.checks.as_slice()) {
            return Err("Retry check evidence disagrees");
        }
        outcome = Some(attempt_outcome);
        checks = Some(&attempt.checks);
    }
    let expected_checks = run
        .manifest
        .cases
        .iter()
        .map(|case| case.case.checks.len())
        .sum::<usize>();
    let checks = checks.unwrap_or_default();
    if checks.len() != expected_checks {
        return Err("Retained check evidence is incomplete");
    }
    if checks
        .iter()
        .any(|check| !matches!(check.outcome, Outcome::Passed | Outcome::Failed))
    {
        return Err("Check evidence is inconclusive");
    }
    let checks_failed = checks.iter().any(|check| check.outcome == Outcome::Failed);
    if (outcome == Some(Outcome::Passed) && checks_failed)
        || (outcome == Some(Outcome::Failed) && !checks_failed)
    {
        return Err("Run outcome disagrees with retained check evidence");
    }
    Ok(CleanResult {
        outcome: outcome.expect("attempts are non-empty"),
        checks: checks.to_vec(),
    })
}

pub async fn validate_baseline(
    db: &impl ConnectionTrait,
    app: Uuid,
    baseline_id: Uuid,
    current: &RunManifest,
) -> ApiResult<()> {
    let row = one(
        db,
        "SELECT app_id FROM execution_runs WHERE id=$1",
        vec![baseline_id.into()],
    )
    .await?;
    if field::<Uuid>(&row, "app_id")? != app {
        return Err(ApiFailure::missing());
    }
    let baseline = runs::detail(db, baseline_id).await?;
    if let Some(reason) = context_mismatch(&baseline.manifest, current) {
        return Err(ApiFailure::invalid(reason));
    }
    clean_result(&baseline).map_err(ApiFailure::invalid)?;
    Ok(())
}

pub async fn candidates(
    db: &impl ConnectionTrait,
    app: Uuid,
    query: BaselineCandidateQuery,
) -> ApiResult<BaselineCandidateResponse> {
    let current = runs::preview_saved_case(
        db,
        app,
        SavedCasePreviewQuery {
            build_id: query.build_id,
            case_version_id: query.case_version_id,
            profile_id: query.profile_id,
        },
    )
    .await?
    .manifest
    .ok_or_else(|| ApiFailure::invalid("Saved test is not ready to run"))?;
    if current.environment_revision != query.environment_revision {
        return Err(super::execution_store::conflict(
            "Environment changed; refresh the run choices",
        ));
    }
    let mut items = Vec::new();
    let mut cursor: Option<(DateTime<Utc>, Uuid)> = None;
    loop {
        let batch = if let Some((created_at, id)) = cursor {
            rows(
                db,
                "SELECT r.id,r.created_at,b.original_filename FROM execution_runs r
                 JOIN builds b ON b.id=r.build_id
                 WHERE r.app_id=$1 AND r.source_kind='saved_case'
                   AND (r.created_at,r.id)<($2,$3)
                 ORDER BY r.created_at DESC,r.id DESC LIMIT 100",
                vec![app.into(), created_at.into(), id.into()],
            )
            .await?
        } else {
            rows(
                db,
                "SELECT r.id,r.created_at,b.original_filename FROM execution_runs r
                 JOIN builds b ON b.id=r.build_id
                 WHERE r.app_id=$1 AND r.source_kind='saved_case'
                 ORDER BY r.created_at DESC,r.id DESC LIMIT 100",
                vec![app.into()],
            )
            .await?
        };
        let Some(last) = batch.last() else {
            break;
        };
        cursor = Some((field(last, "created_at")?, field(last, "id")?));
        for row in batch {
            let run = runs::detail(db, field(&row, "id")?).await?;
            if context_mismatch(&run.manifest, &current).is_some() {
                continue;
            }
            let Ok(result) = clean_result(&run) else {
                continue;
            };
            items.push(BaselineCandidate {
                run_id: run.id,
                build_id: run.manifest.build_id,
                build_name: field(&row, "original_filename")?,
                build_sha256: run.manifest.build_sha256,
                created_at: run.created_at,
                outcome: result.outcome,
            });
            if items.len() == 20 {
                return Ok(BaselineCandidateResponse { items });
            }
        }
    }
    Ok(BaselineCandidateResponse { items })
}

pub async fn comparison(
    db: &impl ConnectionTrait,
    run_id: Uuid,
) -> ApiResult<RunComparisonResponse> {
    let current = runs::detail(db, run_id).await?;
    if current.state != JobState::Finished {
        return Ok(RunComparisonResponse {
            run_id,
            baseline_run_id: current.baseline_run_id,
            label: ComparisonLabel::ComparisonPending,
            reason: None,
            checks: Vec::new(),
        });
    }
    let Some(baseline_id) = current.baseline_run_id else {
        return Ok(RunComparisonResponse {
            run_id,
            baseline_run_id: None,
            label: ComparisonLabel::NoBaseline,
            reason: None,
            checks: Vec::new(),
        });
    };
    let baseline = runs::detail(db, baseline_id).await?;
    let not_comparable = |reason: &str| RunComparisonResponse {
        run_id,
        baseline_run_id: Some(baseline_id),
        label: ComparisonLabel::NotComparable,
        reason: Some(reason.into()),
        checks: Vec::new(),
    };
    if let Some(reason) = context_mismatch(&baseline.manifest, &current.manifest) {
        return Ok(not_comparable(reason));
    }
    let baseline_result = match clean_result(&baseline) {
        Ok(result) => result,
        Err(reason) => return Ok(not_comparable(reason)),
    };
    let current_result = match clean_result(&current) {
        Ok(result) => result,
        Err(reason) => return Ok(not_comparable(reason)),
    };
    let baseline_checks: BTreeMap<_, _> = baseline_result
        .checks
        .iter()
        .map(|check| (check.check_id.as_str(), check))
        .collect();
    let current_checks: BTreeMap<_, _> = current_result
        .checks
        .iter()
        .map(|check| (check.check_id.as_str(), check))
        .collect();
    if baseline_checks.keys().ne(current_checks.keys()) {
        return Ok(not_comparable("Check identity changed"));
    }
    let checks = current_checks
        .iter()
        .map(|(id, current_check)| {
            let baseline_check = baseline_checks[id];
            CheckComparison {
                check_id: (*id).into(),
                expected: current_check.expected.clone(),
                baseline_observed: baseline_check.observed.clone(),
                current_observed: current_check.observed.clone(),
                baseline_reason: baseline_check.reason.clone(),
                current_reason: current_check.reason.clone(),
                baseline_artifact_ids: baseline_check.artifact_ids.clone(),
                current_artifact_ids: current_check.artifact_ids.clone(),
                baseline_outcome: baseline_check.outcome,
                current_outcome: current_check.outcome,
            }
        })
        .collect();
    let label = comparison_label(
        baseline_result.outcome,
        current_result.outcome,
        baseline.manifest.build_sha256 == current.manifest.build_sha256,
    );
    Ok(RunComparisonResponse {
        run_id,
        baseline_run_id: Some(baseline_id),
        label,
        reason: None,
        checks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparison_labels_cover_clean_transitions() {
        assert_eq!(
            comparison_label(Outcome::Passed, Outcome::Failed, false),
            ComparisonLabel::Regression
        );
        assert_eq!(
            comparison_label(Outcome::Passed, Outcome::Failed, true),
            ComparisonLabel::NewFailureSameBuild
        );
        assert_eq!(
            comparison_label(Outcome::Failed, Outcome::Failed, false),
            ComparisonLabel::StillFailing
        );
        assert_eq!(
            comparison_label(Outcome::Failed, Outcome::Passed, false),
            ComparisonLabel::Recovered
        );
        assert_eq!(
            comparison_label(Outcome::Passed, Outcome::Passed, false),
            ComparisonLabel::Unchanged
        );
    }
}
