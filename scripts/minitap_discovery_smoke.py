"""Explicit real SDK/device acceptance on the qualified sample; never invoked by CI."""

import copy
import json
import uuid

from app_setup_smoke import request
from task_session_smoke import main


def scenario(owner, csrf, app, path, wait_for, directory):
    ready = request(owner, "GET", path)
    job = str(uuid.uuid4())
    request(
        owner,
        "POST",
        f"/api/apps/{app}/test-generations",
        {
            "id": job,
            "session_id": ready["id"],
            "expected_revision": ready["revision"],
            "engine": "minitap_v1",
            "category": "smoke",
            "journey": 'Replace the task input with "Hello", tap Save, and observe the saved Hello task. Stop there.',
            "allow_writes": True,
            "reuse_job_id": None,
        },
        csrf,
    )
    result = wait_for(
        lambda s: any(
            t["id"] == job and t["state"] in ("completed", "failed") for t in s["tasks"]
        )
    )
    discovery = next(t for t in result["tasks"] if t["id"] == job)
    progress = discovery["progress"]
    (directory / "discovery.json").write_text(json.dumps(discovery, indent=2))
    assert discovery["state"] == "completed" and progress["proposals"], discovery[
        "message"
    ]
    assert progress["engine"] == "minitap_v1" and 1 < progress["usage"]["calls"] <= 16
    assert not progress["usage"]["unknown_calls"]
    candidate = max(progress["proposals"], key=lambda p: len(p["sequence"]["actions"]))
    sequence = copy.deepcopy(candidate["sequence"])
    assert all(a["kind"] == "direct" for a in sequence["actions"])
    assert any(
        a["command"]["operation"] == "set_text" and a["command"]["text"] == "Hello"
        for a in sequence["actions"]
    )
    # Independent acceptance expectation, supplied by this operator fixture, never by the model.
    sequence["checks"] = [
        {
            "id": "saved",
            "checkpoint_id": sequence["actions"][-1]["checkpoint_id"],
            "description": "Saved Hello is visible",
            "method": "ui_property_equals_v1",
            "resource_id": "ai.mobileqa.demo:id/task_row",
            "ready_resource_id": "ai.mobileqa.demo:id/task_row",
            "text_filter": "",
            "property": "text",
            "expected": "Hello",
            "prerequisite_check_ids": [],
            "required": True,
            "observation_seconds": 2,
        }
    ]
    saved = request(
        owner,
        "POST",
        f"/api/apps/{app}/test-library/from-recording",
        {
            "mutation_id": str(uuid.uuid4()),
            "source_task_id": job,
            "expectations_confirmed": True,
            "tests": [
                {
                    "template_id": None,
                    "proposal_id": candidate["id"],
                    "title": candidate["title"],
                    "requirement": "Saving Hello displays Hello",
                    "sequence": sequence,
                }
            ],
        },
        csrf,
    )
    draft = request(
        owner, "GET", f"/api/apps/{app}/test-library/{saved['entry_ids'][0]}/draft"
    )
    assert draft["definition"]["content"]["actions"] == sequence["actions"]
    results = [discovery]
    for wrong in (False, True):
        current = request(owner, "GET", path)
        command = str(uuid.uuid4())
        trial = copy.deepcopy(sequence)
        if wrong:
            trial["checks"][0]["expected"] = "This value must not exist"
        request(
            owner,
            "POST",
            path + "/commands",
            {
                "id": command,
                "expected_revision": current["revision"],
                "frame_id": None,
                "title": "Direct replay with independent assertion",
                "sequence": trial,
            },
            csrf,
        )
        done = wait_for(
            lambda s: any(
                t["id"] == command and t["state"] in ("completed", "failed")
                for t in s["tasks"]
            )
        )
        execution = next(t for t in done["tasks"] if t["id"] == command)
        assert execution["state"] == ("failed" if wrong else "completed")
        assert not execution.get("generation") and not execution.get("progress")
        assert (
            next(t for t in done["tasks"] if t["id"] == job)["progress"]["usage"]
            == progress["usage"]
        )
        results.append(execution)
    (directory / "qualification.json").write_text(
        json.dumps(
            {
                "saved_entry_ids": saved["entry_ids"],
                "discovery_usage": progress["usage"],
                "replay": "direct actions only; positive and negative independent assertions",
                "initial_state": "same leased sample app; clean-reset replay and arbitrary APK qualification remain separate gates",
            },
            indent=2,
        )
    )
    return results


if __name__ == "__main__":
    main(scenario)
