"""Materialize a model-selected journal prefix; models never manufacture replay provenance."""

from uuid import uuid4

from mobile_qa_worker.generated.models import (
    AuthoringModelRequest2,
    DiscoveryDraftBatch,
    ProposalBatch,
)
from mobile_qa_worker.qualification.config import QualificationError


def materialize(request: AuthoringModelRequest2, drafts: DiscoveryDraftBatch) -> ProposalBatch:
    journal = request.journal or []
    proposals: list[dict[str, object]] = []
    for draft in drafts.proposals:
        path = journal[: draft.through_action]
        if len(path) != draft.through_action or any(r.outcome.value != "completed" for r in path):
            raise QualificationError("proposal_path_not_observed")
        sources = {s.id: s for s in request.snapshots}
        if any(r.before_id not in sources or r.after_id not in sources for r in path):
            raise QualificationError("proposal_source_not_observed")
        checks: list[dict[str, object]] = []
        for check in draft.checks:
            if check.after_action > len(path):
                raise QualificationError("proposal_check_not_observed")
            receipt = path[check.after_action - 1]
            if receipt.after_id is None:
                raise QualificationError("proposal_check_not_observed")
            controls = sources[receipt.after_id].frame.controls
            if check.method.value == "manual" or any(
                not any(c.resource_id == target for c in controls)
                for target in (check.resource_id, check.ready_resource_id)
            ):
                raise QualificationError("proposal_check_not_observed")
            checks.append(
                {
                    **check.model_dump(mode="json", exclude={"after_action"}),
                    "id": f"check-{len(checks) + 1}",
                    "checkpoint_id": str(receipt.id),
                    "prerequisite_check_ids": [],
                }
            )
        proposals.append(
            {
                "id": str(uuid4()),
                "title": draft.title,
                "category": request.category.value,
                "requirement": draft.requirement,
                "questions": draft.questions,
                "path_ids": [str(r.id) for r in path],
                "source_ids": list(
                    dict.fromkeys(str(id) for r in path for id in (r.before_id, r.after_id))
                ),
                "sequence": {
                    "actions": [
                        {
                            "id": str(r.id),
                            "checkpoint_id": str(r.id),
                            "kind": "direct",
                            "instruction": "",
                            "command": r.command.model_dump(mode="json"),
                        }
                        for r in path
                    ],
                    "checks": checks,
                },
            }
        )
    return ProposalBatch.model_validate({"proposals": proposals})
