"""Author/review/pin coverage using real browser HTTP routes, then run the existing
fake Python worker across API restarts. This proves protocol persistence, not UI
rendering or real Android execution. Operator setup is confined to fixture grants,
profiles and worker identity in execution_smoke.
"""

import uuid
from execution_smoke import main as execution_main
from app_setup_smoke import request


def mutate(owner, csrf, method, path, body):
    payload = {"mutation_id": str(uuid.uuid4()), **body}
    result = request(owner, method, path, payload, csrf)
    # Simulate a lost response: replay before the next action must be identical.
    assert request(owner, method, path, payload, csrf) == result
    return result


def main():
    saved = {}

    def author(owner, csrf, app, profile):
        base = f"/api/apps/{app}/test-library"

        def create(kind):
            return mutate(
                owner,
                csrf,
                "POST",
                base,
                {
                    "entry_id": str(uuid.uuid4()),
                    "kind": kind,
                    "key": f"http-{kind}-{uuid.uuid4()}",
                    "template_profile_id": profile,
                },
            )

        def publish(draft):
            path = base + "/" + draft["entry"]["id"]
            candidate = mutate(
                owner,
                csrf,
                "POST",
                path + "/submit",
                {
                    "expected_revision": draft["entry"]["revision"],
                },
            )
            for purpose in ["business", "executability"]:
                candidate = mutate(
                    owner,
                    csrf,
                    "POST",
                    path + "/versions/" + candidate["version"]["id"] + "/review",
                    {
                        "expected_revision": candidate["entry"]["revision"],
                        "content_hash": candidate["version"]["content_hash"],
                        "purpose": purpose,
                        "decision": "approve",
                        "reason": None,
                    },
                )
            assert candidate["review_state"] == "approved"
            return candidate

        case = create("case")
        assert case["definition"]["content"]["provenance"] == "user_authored"
        # Explicit browser save, even when starting from the optional demo template.
        case = mutate(
            owner,
            csrf,
            "PUT",
            base + "/" + case["entry"]["id"] + "/draft",
            {
                "expected_revision": case["entry"]["revision"],
                "definition": case["definition"],
            },
        )
        case = publish(case)
        saved["case"] = case
        selection = {
            "case_version_id": case["version"]["id"],
            "data_variant": "default",
            "required": True,
        }
        suite = create("suite")
        suite["definition"]["content"].update(
            title="Persistence checks", cases=[selection]
        )
        suite = mutate(
            owner,
            csrf,
            "PUT",
            base + "/" + suite["entry"]["id"] + "/draft",
            {
                "expected_revision": suite["entry"]["revision"],
                "definition": suite["definition"],
            },
        )
        suite = publish(suite)
        plan = create("plan")
        plan["definition"]["content"].update(
            title="Reviewed release checks",
            suite_version_ids=[suite["version"]["id"]],
            cases=[selection],
            budget={
                "duration_seconds": 1200,
                "max_steps": 30,
                "artifact_bytes": 16777216,
            },
        )
        plan = mutate(
            owner,
            csrf,
            "PUT",
            base + "/" + plan["entry"]["id"] + "/draft",
            {
                "expected_revision": plan["entry"]["revision"],
                "definition": plan["definition"],
            },
        )
        assert len(plan["coverage"]["cases"]) == 1, (
            "Direct + suite reference must resolve once"
        )
        assert plan["coverage"]["required_count"] == 1
        plan = publish(plan)
        return plan["version"]["id"]

    def edit(owner, csrf, app):
        case = saved["case"]
        base = f"/api/apps/{app}/test-library/{case['entry']['id']}"
        draft = mutate(
            owner,
            csrf,
            "POST",
            base + "/draft",
            {
                "expected_revision": case["entry"]["revision"],
                "source_version_id": case["version"]["id"],
            },
        )
        draft["definition"]["content"]["title"] = (
            "Later work must not alter the queued run"
        )
        draft = mutate(
            owner,
            csrf,
            "PUT",
            base + "/draft",
            {
                "expected_revision": draft["entry"]["revision"],
                "definition": draft["definition"],
            },
        )
        assert draft["definition"]["content"]["version"] == 2
        historical = request(owner, "GET", base + "/versions/" + case["version"]["id"])
        assert historical["version"] == case["version"]

    execution_main(author=author, edit_after_queue=edit)
    print(
        "Test library HTTP acceptance passed: draft/save/replay, case + suite + plan review, explicit default, unique coverage, frozen queued manifest after editing and API restart; simulated worker only"
    )


if __name__ == "__main__":
    main()
