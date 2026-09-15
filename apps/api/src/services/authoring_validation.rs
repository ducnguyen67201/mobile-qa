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
        if old
            .generation
            .as_ref()
            .is_some_and(|g| g.engine == Some(DiscoveryEngine::MinitapV1))
        {
            discovery_progress(
                old.progress.as_ref(),
                p,
                package,
                old.generation.as_ref().is_some_and(|g| g.allow_writes),
            )?;
        }
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
            ))
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

/// A receipt cannot claim an effect before a previously acknowledged intent.
fn discovery_progress(
    old: Option<&GenerationProgress>,
    p: &GenerationProgress,
    package: &str,
    allow_writes: bool,
) -> ApiResult<()> {
    if p.engine != Some(DiscoveryEngine::MinitapV1)
        || p.journal.len() > 12
        || p.usage.unknown_calls > p.usage.calls
    {
        return Err(conflict("Invalid discovery engine or journal"));
    }
    let previous = old.map(|v| v.journal.as_slice()).unwrap_or_default();
    if p.journal.len() < previous.len() || p.journal.len() > previous.len() + 1 {
        return Err(conflict("Persist each discovery intent separately"));
    }
    if let Some(before) = old {
        if before.engine != p.engine || before.source_job_id != p.source_job_id {
            return Err(conflict("Discovery provenance cannot change"));
        }
        let legal = before.state == p.state
            || matches!(
                (&before.state, &p.state),
                (
                    GenerationState::Queued,
                    GenerationState::Discovering
                        | GenerationState::Drafting
                        | GenerationState::Failed
                        | GenerationState::Canceled
                ) | (
                    GenerationState::Discovering,
                    GenerationState::Drafting | GenerationState::Failed | GenerationState::Canceled
                ) | (
                    GenerationState::Drafting,
                    GenerationState::Ready
                        | GenerationState::NeedsInput
                        | GenerationState::Failed
                        | GenerationState::Canceled
                )
            );
        if !legal {
            return Err(conflict("Invalid discovery phase transition"));
        }
    }
    let sources: std::collections::BTreeSet<_> = p.snapshots.iter().map(|s| s.id).collect();
    let source_order: std::collections::BTreeMap<_, _> = p
        .snapshots
        .iter()
        .enumerate()
        .map(|(index, s)| (s.id, index))
        .collect();
    let mut ids = std::collections::BTreeSet::new();
    for (index, r) in p.journal.iter().enumerate() {
        r.command.validate(package).map_err(ApiFailure::invalid)?;
        if !allow_writes && !matches!(r.command, DirectCommand::WaitFor { .. }) {
            return Err(conflict("This discovery only permits observation"));
        }
        if !ids.insert(r.id)
            || !sources.contains(&r.before_id)
            || r.after_id.is_some_and(|id| !sources.contains(&id))
            || (r.outcome == DiscoveryOutcome::Completed && r.after_id.is_none())
            || (r.outcome == DiscoveryOutcome::Pending && r.after_id.is_some())
        {
            return Err(conflict("Invalid discovery receipt"));
        }
        if r.after_id
            .is_some_and(|id| source_order.get(&id) < source_order.get(&r.before_id))
        {
            return Err(conflict("Discovery observations cannot move backwards"));
        }
        // Passive redraws between operations are observations, not omitted device actions.
        if index > 0
            && (p.journal[index - 1].outcome != DiscoveryOutcome::Completed
                || p.journal[index - 1]
                    .after_id
                    .and_then(|id| source_order.get(&id))
                    > source_order.get(&r.before_id))
        {
            return Err(conflict("Discovery path is not continuous"));
        }
        match previous.get(index) {
            None if r.outcome != DiscoveryOutcome::Pending => {
                return Err(conflict("Acknowledge intent before device work"))
            }
            Some(before)
                if before != r
                    && (before.outcome != DiscoveryOutcome::Pending
                        || before.id != r.id
                        || before.before_id != r.before_id
                        || before.command != r.command) =>
            {
                return Err(conflict("Discovery history cannot change"))
            }
            _ => (),
        }
    }
    let completed: Vec<_> = p
        .journal
        .iter()
        .filter(|r| r.outcome == DiscoveryOutcome::Completed)
        .collect();
    if p.trace
        != completed
            .iter()
            .map(|r| r.command.clone())
            .collect::<Vec<_>>()
    {
        return Err(conflict("Trace must match completed discovery receipts"));
    }
    if matches!(
        p.state,
        GenerationState::Ready | GenerationState::NeedsInput
    ) && p
        .journal
        .iter()
        .any(|r| r.outcome == DiscoveryOutcome::Pending)
    {
        return Err(conflict("Discovery operation remains pending"));
    }
    for proposal in &p.proposals {
        let n = proposal.sequence.actions.len();
        if n == 0
            || n > completed.len()
            || proposal.path_ids != completed[..n].iter().map(|r| r.id).collect::<Vec<_>>()
        {
            return Err(conflict("Proposal must include its observed setup path"));
        }
        for (action, receipt) in proposal.sequence.actions.iter().zip(&completed) {
            if action.kind != mobile_qa_contracts::execution::ActionKind::Direct
                || action.command.as_ref() != Some(&receipt.command)
                || !proposal.source_ids.contains(&receipt.before_id)
                || !receipt
                    .after_id
                    .is_some_and(|id| proposal.source_ids.contains(&id))
            {
                return Err(conflict("Proposal differs from the observed path"));
            }
        }
    }
    Ok(())
}
