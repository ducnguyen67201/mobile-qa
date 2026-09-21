//! Pure comparisons of immutable, verified facts. No scheduling or UI inference.
use mobile_qa_contracts::{execution::*, regression::*};

pub fn context_matches(a: &RunManifest, b: &RunManifest) -> bool {
    a.app_id == b.app_id
        && a.environment_revision == b.environment_revision
        && a.profile == b.profile
        && a.resolved_model == b.resolved_model
}
pub fn outcome(run: &RunResponse, case: &ResolvedCase) -> Option<Outcome> {
    if run.state != JobState::Finished || run.manifest.profile.execution_context.is_none() {
        return None;
    }
    if run
        .manifest
        .cases
        .iter()
        .filter(|c| c.definition_id == case.definition_id)
        .count()
        != 1
    {
        return None;
    }
    let attempts: Vec<_> = run
        .attempts
        .iter()
        .filter(|a| a.case_version_id == case.definition_id)
        .collect();
    let first = attempts.first()?.outcome?;
    if !matches!(first, Outcome::Passed | Outcome::Failed) {
        return None;
    }
    for a in attempts {
        if a.outcome != Some(first)
            || a.state != JobState::Finished
            || !a.preflight.as_ref().is_some_and(|p| {
                Some(&p.context) == run.manifest.profile.execution_context.as_ref()
                    && p.attempt_id == a.id
                    && p.build_sha256 == run.manifest.build_sha256
            })
            || a.cleanup != CleanupState::VerifiedClean
            || !a.recovery_events.is_empty()
            || !a
                .original_cleanup
                .as_ref()
                .is_some_and(|r| r.stopped && r.reset == CleanupState::VerifiedClean)
        {
            return None;
        }
        let verified = |c: &CheckResult| {
            (c.observed.is_some() || c.observation_kind == Some(ObservationKind::Absent))
                && !c.artifact_ids.is_empty()
                && c.artifact_ids.iter().all(|id| {
                    a.artifacts
                        .iter()
                        .any(|f| f.id == *id && f.state == EvidenceState::Sealed)
                })
        };
        if first == Outcome::Passed {
            let required: Vec<_> = case.case.checks.iter().filter(|c| c.required).collect();
            if required.is_empty()
                || required.iter().any(|c| {
                    !a.checks
                        .iter()
                        .any(|r| r.check_id == c.id && r.outcome == Outcome::Passed && verified(r))
                })
            {
                return None;
            }
        } else if !a
            .checks
            .iter()
            .any(|c| c.outcome == Outcome::Failed && verified(c))
        {
            return None;
        }
    }
    Some(first)
}
pub fn eligible(current: &RunManifest, baseline: &RunResponse) -> bool {
    context_matches(current, &baseline.manifest)
        && current.cases.iter().any(|c| {
            baseline.manifest.cases.iter().any(|b| {
                b.case.key == c.case.key
                    && b.data_variant == c.data_variant
                    && b.definition_id == c.definition_id
                    && b.content_hash == c.content_hash
                    && outcome(baseline, b).is_some()
            })
        })
}
pub fn transition(before: Outcome, after: Outcome, different_build: bool) -> ComparisonKind {
    match (before, after) {
        (Outcome::Passed, Outcome::Failed) if different_build => ComparisonKind::Regression,
        (Outcome::Passed, Outcome::Failed) => ComparisonKind::NewFailure,
        (Outcome::Failed, Outcome::Failed) => ComparisonKind::StillFailing,
        (Outcome::Failed, Outcome::Passed) => ComparisonKind::Recovered,
        (Outcome::Passed, Outcome::Passed) => ComparisonKind::Unchanged,
        _ => ComparisonKind::NotComparable,
    }
}
pub fn compare(current: &RunResponse, baseline: Option<&RunResponse>) -> RunComparison {
    let mut cases = vec![];
    for c in &current.manifest.cases {
        let b = baseline.and_then(|r| {
            r.manifest
                .cases
                .iter()
                .find(|b| b.case.key == c.case.key && b.data_variant == c.data_variant)
        });
        let after = outcome(current, c);
        let before = baseline.zip(b).and_then(|(r, c)| outcome(r, c));
        let (kind, reason) = match (baseline, b) {
            (None, _) => (ComparisonKind::NoBaseline, "No baseline selected"),
            (Some(_), None) => (
                ComparisonKind::Added,
                "Test was not present in the baseline",
            ),
            (Some(r), Some(_)) if !context_matches(&current.manifest, &r.manifest) => (
                ComparisonKind::NotComparable,
                "Environment or execution configuration changed",
            ),
            (Some(_), Some(b))
                if b.definition_id != c.definition_id || b.content_hash != c.content_hash =>
            {
                (
                    ComparisonKind::NotComparable,
                    "Test version or expectations changed",
                )
            }
            (Some(r), Some(_)) => match (before, after) {
                (Some(x), Some(y)) => (
                    transition(
                        x,
                        y,
                        current.manifest.build_sha256 != r.manifest.build_sha256,
                    ),
                    "Compared the same saved test and execution context",
                ),
                _ => (
                    ComparisonKind::NotComparable,
                    "Missing clean start, cleanup, conclusive checks, or consistent attempts",
                ),
            },
        };
        let checks = |r: &RunResponse, id| {
            r.attempts
                .iter()
                .filter(|a| a.case_version_id == id)
                .flat_map(|a| a.checks.clone())
                .collect()
        };
        cases.push(CaseComparison {
            case_key: c.case.key.clone(),
            title: c.case.title.clone(),
            data_variant: c.data_variant.clone(),
            kind,
            reason: reason.into(),
            baseline_case_id: b.map(|v| v.definition_id),
            current_case_id: Some(c.definition_id),
            baseline_outcome: before,
            current_outcome: after,
            baseline_checks: baseline
                .zip(b)
                .map(|(r, b)| checks(r, b.definition_id))
                .unwrap_or_default(),
            current_checks: checks(current, c.definition_id),
        });
    }
    if let Some(baseline) = baseline {
        for b in &baseline.manifest.cases {
            if !current
                .manifest
                .cases
                .iter()
                .any(|c| c.case.key == b.case.key && c.data_variant == b.data_variant)
            {
                cases.push(CaseComparison {
                    case_key: b.case.key.clone(),
                    title: b.case.title.clone(),
                    data_variant: b.data_variant.clone(),
                    kind: ComparisonKind::Removed,
                    reason: "Test is no longer in this run".into(),
                    baseline_case_id: Some(b.definition_id),
                    current_case_id: None,
                    baseline_outcome: outcome(baseline, b),
                    current_outcome: None,
                    baseline_checks: vec![],
                    current_checks: vec![],
                });
            }
        }
    }
    RunComparison {
        policy_version: 1,
        baseline_run_id: baseline.map(|b| b.id),
        cases,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transitions_do_not_infer_regressions_from_blockers() {
        use ComparisonKind::*;
        use Outcome::*;
        for (a, b, d, k) in [
            (Passed, Failed, true, Regression),
            (Passed, Failed, false, NewFailure),
            (Failed, Failed, true, StillFailing),
            (Failed, Passed, true, Recovered),
            (Passed, Passed, true, Unchanged),
            (Blocked, Failed, true, NotComparable),
            (Passed, Inconclusive, true, NotComparable),
            (Canceled, Passed, true, NotComparable),
        ] {
            assert_eq!(transition(a, b, d), k);
        }
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;
    use uuid::Uuid;
    fn passed() -> RunResponse {
        serde_json::from_str(include_str!(
            "../../tests/fixtures/execution/comparison.json"
        ))
        .unwrap()
    }
    fn failed() -> RunResponse {
        let mut r = passed();
        r.id = Uuid::new_v4();
        r.manifest.build_id = Uuid::new_v4();
        r.manifest.build_sha256 = "f".repeat(64);
        r.attempts[0].preflight.as_mut().unwrap().build_sha256 = r.manifest.build_sha256.clone();
        r.attempts[0].outcome = Some(Outcome::Failed);
        r.attempts[0].checks[0].outcome = Outcome::Failed;
        r.attempts[0].checks[0].observed = Some("".into());
        r
    }
    #[test]
    fn compares_verified_facts_and_freezes_baseline_identity() {
        let b = passed();
        let f = failed();
        let c = compare(&f, Some(&b));
        assert_eq!(c.baseline_run_id, Some(b.id));
        assert_eq!(c.cases[0].kind, ComparisonKind::Regression);
        assert_eq!(
            compare(&b, Some(&f)).cases[0].kind,
            ComparisonKind::Recovered
        );
        assert_eq!(
            compare(&f, Some(&f)).cases[0].kind,
            ComparisonKind::StillFailing
        );
        assert_eq!(compare(&b, None).cases[0].kind, ComparisonKind::NoBaseline);
    }
    #[test]
    fn uncertain_or_changed_inputs_cannot_be_regressions() {
        let b = passed();
        for mutation in 0..11 {
            let mut f = failed();
            match mutation {
                0 => f.manifest.environment_revision += 1,
                1 => f.manifest.cases[0].content_hash = "changed".into(),
                2 => f.attempts[0].preflight = None,
                3 => f.attempts[0].cleanup = CleanupState::Quarantined,
                4 => f.attempts[0].original_cleanup = None,
                5 => f.attempts[0].checks[0].artifact_ids.clear(),
                6 => f.attempts[0].checks[0].observed = None,
                7 => f.attempts[0].outcome = Some(Outcome::Blocked),
                8 => f.attempts.push(b.attempts[0].clone()),
                9 => f.attempts[0].preflight.as_mut().unwrap().context.locale = "changed".into(),
                _ => {
                    f.manifest.profile.model = Some(
                        mobile_qa_contracts::model_registry::ModelBinding::Legacy("changed".into()),
                    )
                }
            };
            assert_eq!(
                compare(&f, Some(&b)).cases[0].kind,
                ComparisonKind::NotComparable,
                "mutation {mutation}"
            );
        }
    }
    #[test]
    fn proven_absence_is_distinct_from_null_observation() {
        let mut f = failed();
        f.attempts[0].checks[0].observed = None;
        f.attempts[0].checks[0].observation_kind = Some(ObservationKind::Absent);
        assert_eq!(
            compare(&f, Some(&passed())).cases[0].kind,
            ComparisonKind::Regression
        );
    }
    #[test]
    fn coverage_changes_remain_visible() {
        let b = passed();
        let mut f = failed();
        f.manifest.cases[0].case.key = "other".into();
        let result = compare(&f, Some(&b));
        assert_eq!(result.cases[0].kind, ComparisonKind::Added);
        assert_eq!(result.cases[1].kind, ComparisonKind::Removed);
    }
    #[test]
    fn manifest_legacy_serialization_is_unchanged() {
        let mut raw = serde_json::to_value(passed().manifest).unwrap();
        raw.as_object_mut().unwrap().remove("source");
        raw["plan_version_id"] = serde_json::json!(Uuid::new_v4());
        raw["plan_hash"] = serde_json::json!("legacy");
        let decoded: RunManifest = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), raw);
    }
}
