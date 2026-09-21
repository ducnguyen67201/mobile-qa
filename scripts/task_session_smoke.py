"""Explicit real API → Minitap acceptance. Never called by ordinary check commands."""

import argparse
import base64
import json
import os
import secrets
import subprocess
import time
import tomllib
import uuid
from pathlib import Path

from app_setup_smoke import (
    BASE,
    ROOT,
    client,
    database,
    google_fixture,
    google_sign_in,
    provision,
    request,
    server,
)
from execution_smoke import task


def main(scenario=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--apk", type=Path, required=True)
    parser.add_argument("--model-definition", type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)
    run = str(uuid.uuid4())
    directory = ROOT / ".private" / "task-session-acceptance" / run
    directory.mkdir(parents=True, mode=0o700)
    profile = tomllib.loads(args.profile.read_text())
    model_definition = json.loads(args.model_definition.read_text())
    if profile.get("model_ref") != model_definition.get("reference"):
        raise SystemExit("Host model_ref must match the registered model definition")
    # Own isolated state; never rewrite the operator's existing profile or AVD.
    profile["state_root"] = str(directory / "phone-state")
    host_profile = directory / "host.toml"
    model_ref = profile.pop("model_ref", None)
    profile_lines = [f"{k} = {json.dumps(v)}" for k, v in profile.items()]
    if model_ref is not None:
        profile_lines.extend(
            [
                "",
                "[model_ref]",
                f"key = {json.dumps(model_ref['key'])}",
                f"revision = {int(model_ref['revision'])}",
            ]
        )
    host_profile.write_text("\n".join(profile_lines) + "\n")
    env = {
        **os.environ,
        "MOBILE_QA_TEST_SCOPE": run,
        "MOBILE_QA_WORKER_TOKEN": secrets.token_urlsafe(48),
    }
    with google_fixture() as (key, jwks), database():
        env.update(
            GOOGLE_CLIENT_ID="synthetic.apps.googleusercontent.com",
            MOBILE_QA_TEST_GOOGLE_JWKS=jwks,
        )
        credentials, org = provision(env)
        with server(env):
            owner = client()
            login = google_sign_in(owner, key, credentials)
            actor, csrf = login["user"]["id"], login["csrf_token"]
            app = request(
                owner,
                "POST",
                "/api/apps",
                {
                    "organization_id": org,
                    "name": "Real task acceptance",
                    "android_package": "ai.mobileqa.demo",
                    "environment_name": "Offline sample",
                    "backend_origins": ["http://127.0.0.1:8765"],
                    "login_origins": [],
                },
                csrf,
            )["id"]
            data = args.apk.read_bytes()
            upload = request(
                owner,
                "POST",
                f"/api/apps/{app}/build-uploads",
                {
                    "original_filename": "qa-tasks-sample.apk",
                    "expected_size": len(data),
                },
                csrf,
            )
            boundary = "phone" + uuid.uuid4().hex
            body = (
                f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="sample.apk"\r\nContent-Type: application/octet-stream\r\n\r\n'.encode()
                + data
                + f"\r\n--{boundary}--\r\n".encode()
            )
            prefix = f"/api/apps/{app}/build-uploads/{upload['id']}"
            request(
                owner,
                "PUT",
                prefix + "/content",
                body,
                csrf,
                "multipart/form-data; boundary=" + boundary,
            )
            build = request(owner, "POST", prefix + "/complete", csrf=csrf)
            assert build["validation"]["state"] == "validated"
            profile_id = str(uuid.uuid4())
            registered = directory / "registered-profile.json"
            registered.write_text(
                json.dumps(
                    {
                        "id": profile_id,
                        "name": "Local Minitap acceptance",
                        "driver": "minitap",
                        "package": "ai.mobileqa.demo",
                        "adapter": "demo_persistence_v1",
                        "device_identity": "local-emulator-5554",
                        "image": profile["system_image"],
                        "model": model_definition["reference"],
                        "qualified": True,
                        "qualification_reference": "existing-local-demo-seam; full campaign remains open",
                        "max_apk_bytes": 104857600,
                    }
                )
            )
            task(
                env, actor, app, "register-model", file=args.model_definition.resolve()
            )
            task(env, actor, app, "register-profile", file=registered)
            task(
                env,
                actor,
                app,
                "register-worker",
                worker=str(uuid.uuid4()),
                profile=profile_id,
            )
            opened = request(
                owner,
                "POST",
                f"/api/apps/{app}/phones",
                {"id": str(uuid.uuid4()), "build_id": None, "profile_id": None},
                csrf,
            )
            phone_id = opened["id"]
            path = f"/api/phones/{phone_id}"
            with (directory / "worker.log").open("wb") as log:
                worker = subprocess.Popen(
                    [
                        "uv",
                        "run",
                        "--no-sync",
                        "--project",
                        "apps/mobile-worker",
                        "--frozen",
                        "mobile-qa-worker",
                        "task-worker",
                        "--origin",
                        BASE,
                        "--state",
                        str(directory / "worker-state"),
                        "--profile",
                        str(host_profile),
                        "--once",
                    ],
                    cwd=ROOT,
                    env=env,
                    stdout=log,
                    stderr=subprocess.STDOUT,
                    start_new_session=True,
                )
                try:

                    def wait_for(predicate, seconds=330):
                        deadline = time.monotonic() + seconds
                        while time.monotonic() < deadline:
                            current = request(owner, "GET", path)
                            if predicate(current):
                                return current
                            if worker.poll() is not None:
                                raise RuntimeError(
                                    "Task worker exited; inspect private log"
                                )
                            if current["state"] in ("quarantined", "closed"):
                                raise RuntimeError("Session ended unexpectedly")
                            time.sleep(2)
                        raise TimeoutError("Task session deadline exceeded")

                    ready = wait_for(lambda s: s["state"] == "ready")
                    assert ready["frame"] and ready["frame"]["controls"]
                    (directory / "initial.png").write_bytes(
                        base64.b64decode(ready["frame"]["png_base64"])
                    )
                    if scenario:
                        results = scenario(owner, csrf, app, path, wait_for, directory)
                    else:
                        goals = [
                            "Enter Buy milk in the task input and tap Save. Leave the saved task visible.",
                            "Replace the text in the task input with Walk the dog, tap Save, and leave Walk the dog visible.",
                        ]
                        results = []
                        for index, goal in enumerate(goals):
                            current = request(owner, "GET", path)
                            selection = None
                            if index == 1:
                                control = next(
                                    c
                                    for c in current["frame"]["controls"]
                                    if c["resource_id"].endswith("task_input")
                                )
                                selection = {
                                    "frame_id": current["frame"]["id"],
                                    "control_id": control["id"],
                                }
                            request(
                                owner,
                                "POST",
                                path + "/tasks",
                                {
                                    "id": str(uuid.uuid4()),
                                    "goal": goal,
                                    "selection": selection,
                                },
                                csrf,
                            )
                            result = wait_for(
                                lambda s: len(s["tasks"]) == index + 1
                                and s["tasks"][-1]["state"] in ("completed", "failed")
                            )
                            results.append(result["tasks"][-1])
                            (directory / f"task-{index + 1}.png").write_bytes(
                                base64.b64decode(result["frame"]["png_base64"])
                            )
                            assert results[-1]["state"] == "completed", results[-1][
                                "message"
                            ]
                            expected = "Buy milk" if index == 0 else "Walk the dog"
                            assert any(
                                c["label"] == expected
                                and c["resource_id"].endswith("task_row")
                                for c in result["frame"]["controls"]
                            ), "Expected saved task missing from device evidence"
                    request(owner, "POST", path + "/stop", csrf=csrf)
                    closed = wait_for(lambda s: s["state"] == "closed", 60)
                    worker.wait(timeout=15)
                    assert worker.returncode == 0
                    (directory / "result.json").write_text(
                        json.dumps(
                            {
                                "session_id": phone_id,
                                "tasks": results,
                                "state": closed["state"],
                                "verification": "Saved text observed in captured UI hierarchy; browser rendering not exercised",
                            },
                            indent=2,
                        )
                    )
                    print(
                        f"Real task session acceptance passed: {directory}", flush=True
                    )
                finally:
                    if worker.poll() is None:
                        request(owner, "POST", path + "/stop", csrf=csrf)
                        worker.terminate()
                        worker.wait(timeout=60)


if __name__ == "__main__":
    main()
