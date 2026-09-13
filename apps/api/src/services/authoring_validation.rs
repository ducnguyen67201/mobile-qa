//! Worker acknowledgements may add observations, never rewrite commands or past evidence.
use super::execution_store::conflict;
use crate::errors::{ApiFailure, ApiResult};
use mobile_qa_contracts::{automation::*, task_sessions::*};
fn same<T: serde::Serialize>(a: &T, b: &T) -> bool {
    serde_json::to_value(a).ok() == serde_json::to_value(b).ok()
}
pub fn task_update(old: &PhoneTask, next: &PhoneTask, package: &str) -> ApiResult<()> {
    if !same(&old.sequence, &next.sequence) || !same(&old.generation, &next.generation) {
        return Err(conflict("Submitted task content cannot change"));
    }
    if let Some(sequence) = &old.sequence {
        if next.steps.len() > sequence.actions.len() || next.steps.len() < old.steps.len() {
            return Err(conflict("Invalid step acknowledgement order"));
        }
        for (i, receipt) in next.steps.iter().enumerate() {
            if receipt.action_id != sequence.actions[i].id || receipt.message.len() > 500 {
                return Err(conflict("Invalid step identity"));
            }
            if i + 1 < next.steps.len() && receipt.state != StepState::Completed {
                return Err(conflict("Previous step has not completed"));
            }
            if let Some(previous) = old.steps.get(i) {
                if previous.state != StepState::Started && previous != receipt {
                    return Err(conflict("A completed step receipt cannot change"));
                }
            }
        }
        if next.steps.len() > old.steps.len()
            && next
                .steps
                .last()
                .is_some_and(|r| r.state != StepState::Started)
        {
            return Err(conflict(
                "Persist a started receipt before performing a step",
            ));
        }
        if next.steps.len() > old.steps.len() + 1 {
            return Err(conflict("Acknowledge each step before executing the next"));
        }
        if next.state == PhoneTaskState::Completed
            && (next.steps.len() != sequence.actions.len()
                || next.steps.iter().any(|s| s.state != StepState::Completed))
        {
            return Err(conflict("Steps have not completed"));
        }
    } else if !next.steps.is_empty() {
        return Err(conflict("This task has no structured steps"));
    }
    if let Some(p) = &next.progress {
        if old.generation.is_none() {
            return Err(conflict("This task is not generation"));
        }
        if p.snapshots.len() > 8
            || p.trace.len() > 12
            || p.proposals.len() > 5
            || p.usage.calls > 16
            || p.gaps.len() > 20
            || p.gaps.iter().any(|s| s.len() > 1000)
        {
            return Err(ApiFailure::invalid("Generation exceeded its bounds"));
        }
        if let Some(before) = &old.progress {
            if p.usage.calls < before.usage.calls
                || p.usage.input_tokens < before.usage.input_tokens
                || p.usage.output_tokens < before.usage.output_tokens
                || p.usage.unknown_calls < before.usage.unknown_calls
                || p.snapshots.len() < before.snapshots.len()
                || p.trace.len() < before.trace.len()
            {
                return Err(conflict("Generation progress cannot move backwards"));
            }
            for (a, b) in before.snapshots.iter().zip(&p.snapshots) {
                if !same(a, b) {
                    return Err(conflict("Source observations cannot change"));
                }
            }
            for (a, b) in before.trace.iter().zip(&p.trace) {
                if a != b {
                    return Err(conflict("Discovery trace cannot change"));
                }
            }
        }
        let mut ids = std::collections::BTreeSet::new();
        for snap in &p.snapshots {
            super::task_sessions::validate_frame(&snap.frame)?;
            if !ids.insert(snap.id) {
                return Err(ApiFailure::invalid("Duplicate source identity"));
            }
        }
        if next.state == PhoneTaskState::Completed
            && (!matches!(
                p.state,
                GenerationState::Ready | GenerationState::NeedsInput
            ) || p.proposals.is_empty())
        {
            return Err(conflict("Generation has no completed proposals"));
        }
        for command in &p.trace {
            command.validate(package).map_err(ApiFailure::invalid)?;
        }
        let mut proposals = std::collections::BTreeSet::new();
        for proposal in &p.proposals {
            if !proposals.insert(proposal.id)
                || proposal.title.trim().is_empty()
                || proposal.title.len() > 200
                || proposal.requirement.len() > 4000
                || proposal.questions.len() > 20
                || proposal.questions.iter().any(|q| q.len() > 1000)
                || proposal.source_ids.is_empty()
                || proposal.source_ids.iter().any(|id| !ids.contains(id))
            {
                return Err(ApiFailure::invalid("Invalid proposal sources or content"));
            }
            proposal
                .sequence
                .validate(package)
                .map_err(ApiFailure::invalid)?;
            if proposal.sequence.uses_ai() {
                return Err(ApiFailure::invalid(
                    "Generated scenarios must use direct actions",
                ));
            }
            for check in &proposal.sequence.checks {
                for target in [&check.resource_id, &check.ready_resource_id] {
                    if !p
                        .snapshots
                        .iter()
                        .filter(|s| proposal.source_ids.contains(&s.id))
                        .any(|s| s.frame.controls.iter().any(|c| &c.resource_id == target))
                    {
                        return Err(ApiFailure::invalid(
                            "A proposed check target was not observed",
                        ));
                    }
                }
            }
            for action in &proposal.sequence.actions {
                if let Some(target) = action.command.as_ref().and_then(DirectCommand::target) {
                    let observed = p
                        .snapshots
                        .iter()
                        .filter(|s| proposal.source_ids.contains(&s.id))
                        .any(|s| {
                            s.frame.controls.iter().any(|c| match target {
                                DirectTarget::ResourceId { value } => &c.resource_id == value,
                                DirectTarget::Description { value } => &c.description == value,
                            })
                        });
                    if !observed {
                        return Err(ApiFailure::invalid("A proposed target was not observed"));
                    }
                }
            }
        }
    } else if old.progress.is_some() {
        return Err(conflict("Generation progress cannot be removed"));
    }
    Ok(())
}
