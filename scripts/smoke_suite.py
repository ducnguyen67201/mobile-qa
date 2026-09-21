"""Two saved browser-authored cases through the real API and fake worker.

Simulated HTTP acceptance only: the local intake APK is not run on a device.
"""

import uuid

from execution_smoke import main as execution_main
from test_library_smoke import mutate


def author(owner, csrf, app, profile):
    base = f"/api/apps/{app}/test-library"
    selections = []
    for title in ("Saved task appears", "Saved task survives restart"):
        draft = mutate(
            owner,
            csrf,
            "POST",
            base,
            {
                "entry_id": str(uuid.uuid4()),
                "kind": "case",
                "key": f"smoke-{uuid.uuid4()}",
                "template_profile_id": profile,
            },
        )
        draft["definition"]["content"]["title"] = title
        draft = mutate(
            owner,
            csrf,
            "PUT",
            base + "/" + draft["entry"]["id"] + "/draft",
            {
                "expected_revision": draft["entry"]["revision"],
                "definition": draft["definition"],
            },
        )
        assert draft["saved_version_id"] and not draft["issues"], draft
        selections.append(
            {
                "case_version_id": draft["saved_version_id"],
                "data_variant": "default",
                "required": True,
            }
        )
    suite = mutate(
        owner,
        csrf,
        "POST",
        base,
        {
            "entry_id": str(uuid.uuid4()),
            "kind": "suite",
            "key": f"smoke-suite-{uuid.uuid4()}",
            "template_profile_id": profile,
        },
    )
    suite["definition"]["content"].update(
        title="Two independent smoke cases", cases=selections
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
    assert suite["saved_version_id"] and not suite["issues"], suite
    assert [c["definition_id"] for c in suite["coverage"]["cases"]] == [
        item["case_version_id"] for item in selections
    ]
    return suite["saved_version_id"]


if __name__ == "__main__":
    execution_main(author=author, source="suite")
    print(
        "Saved-suite HTTP smoke passed: two independent cases, idempotent queue, "
        "pass/fail/blocked fake attempts, pinned baseline and restart persistence; simulated only"
    )
