"""Secret-free real HTTP/API restart/worker acceptance with explicitly simulated evidence."""

import json
import os
import re
import secrets
import subprocess
import tempfile
import urllib.request
import uuid
from pathlib import Path

from app_setup_smoke import (
    BASE,
    BINARY,
    ORIGIN,
    ROOT,
    client,
    database,
    google_fixture,
    google_sign_in,
    provision,
    request,
    server,
)


def task(env, actor, app, action, **args):
    result = subprocess.run(
        [
            str(BINARY),
            "task",
            "execution",
            f"action:{action}",
            f"actor:{actor}",
            f"app:{app}",
            *[f"{k}:{v}" for k, v in args.items()],
            "--environment",
            "test",
        ],
        cwd=ROOT / "apps/api",
        env=env,
        capture_output=True,
        text=True,
        check=True,
        timeout=30,
    )
    return result.stdout


def import_file(env, actor, app, directory, content):
    path = directory / f"{uuid.uuid4()}.json"
    path.write_text(json.dumps(content))
    output = task(env, actor, app, "import", file=path)
    match = re.search(r"definition_id=([0-9a-f-]+) content_hash=([0-9a-f]+)", output)
    if not match:
        raise RuntimeError("Import did not return definition identity")
    return match[1]


def submit(opener, app, body, csrf):
    req = urllib.request.Request(
        BASE + f"/api/apps/{app}/runs",
        json.dumps(body).encode(),
        {
            "Origin": ORIGIN,
            "Content-Type": "application/json",
            "X-CSRF-Token": csrf,
            "Idempotency-Key": str(uuid.uuid4()),
        },
        method="POST",
    )
    with opener.open(req, timeout=30) as response:
        assert response.status == 201
        return json.load(response)


def main(author=None, edit_after_queue=None):
    os.umask(0o077)
    env = {
        **os.environ,
        "MOBILE_QA_TEST_SCOPE": str(uuid.uuid4()),
        "MOBILE_QA_WORKER_TOKEN": secrets.token_urlsafe(48),
    }
    with (
        google_fixture() as (key, jwks),
        database(),
        tempfile.TemporaryDirectory(prefix="execution-fixtures-") as temporary,
    ):
        directory = Path(temporary)
        env.update(
            GOOGLE_CLIENT_ID="synthetic.apps.googleusercontent.com",
            MOBILE_QA_TEST_GOOGLE_JWKS=jwks,
        )
        credentials, org = provision(env)
        with server(env):
            owner = client()
            session = google_sign_in(owner, key, credentials)
            actor, csrf = session["user"]["id"], session["csrf_token"]
            app = request(
                owner,
                "POST",
                "/api/apps",
                {
                    "organization_id": org,
                    "name": "Synthetic execution",
                    "android_package": "ai.mobileqa.demo",
                    "environment_name": "Controlled fixture",
                    "backend_origins": ["https://synthetic.invalid"],
                    "login_origins": [],
                },
                csrf,
            )["id"]
            data = (ROOT / ".private/test-apks/execution.apk").read_bytes()
            upload = request(
                owner,
                "POST",
                f"/api/apps/{app}/build-uploads",
                {
                    "original_filename": "synthetic-execution.apk",
                    "expected_size": len(data),
                },
                csrf,
            )
            upload_path = f"/api/apps/{app}/build-uploads/{upload['id']}"
            boundary = "execution" + uuid.uuid4().hex
            body = (
                f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="synthetic.apk"\r\nContent-Type: application/octet-stream\r\n\r\n'.encode()
                + data
                + f"\r\n--{boundary}--\r\n".encode()
            )
            request(
                owner,
                "PUT",
                upload_path + "/content",
                body,
                csrf,
                "multipart/form-data; boundary=" + boundary,
            )
            build = request(owner, "POST", upload_path + "/complete", csrf=csrf)
            assert build["validation"]["state"] == "validated"
            profile = str(uuid.uuid4())
            profile_file = directory / "profile.json"
            profile_file.write_text(
                json.dumps(
                    {
                        "id": profile,
                        "name": "Simulated HTTP acceptance",
                        "driver": "fake",
                        "package": "ai.mobileqa.demo",
                        "adapter": "demo_persistence_v1",
                        "device_identity": str(uuid.uuid4()),
                        "image": "synthetic",
                        "model": "none",
                        "qualified": True,
                        "qualification_reference": "synthetic-transport-test",
                        "max_apk_bytes": 262144000,
                    }
                )
            )
            task(env, actor, app, "register-profile", file=profile_file)
            if author is not None:
                plan = author(owner, csrf, app, profile)
            else:
                case = import_file(
                    env,
                    actor,
                    app,
                    directory,
                    json.loads(
                        (
                            ROOT / "contracts/fixtures/execution/persistence-case.json"
                        ).read_text()
                    ),
                )
                plan = import_file(
                    env,
                    actor,
                    app,
                    directory,
                    {
                        "kind": "plan",
                        "content": {
                            "key": "release",
                            "version": 1,
                            "title": "Synthetic release check",
                            "suite_version_ids": [],
                            "cases": [
                                {
                                    "case_version_id": case,
                                    "data_variant": "default",
                                    "required": True,
                                }
                            ],
                            "profile_id": profile,
                            "budget": {
                                "duration_seconds": 1200,
                                "max_steps": 30,
                                "artifact_bytes": 16777216,
                            },
                            "diagnostic_retries": 0,
                            "exclusions": [],
                        },
                    },
                )
            task(
                env,
                actor,
                app,
                "register-worker",
                worker=str(uuid.uuid4()),
                profile=profile,
            )
            request(
                owner,
                "PUT",
                f"/api/apps/{app}/default-test-plan",
                {
                    "mutation_id": str(uuid.uuid4()),
                    "expected_revision": 0,
                    "plan_version_id": plan,
                },
                csrf,
            )
            preview = request(
                owner, "GET", f"/api/apps/{app}/execution-plan?build_id={build['id']}"
            )
            assert not preview["blockers"]
            queued = submit(
                owner,
                app,
                {
                    "build_id": build["id"],
                    "source": {"kind": "release_plan", "plan_version_id": plan},
                    "environment_revision": 1,
                    "baseline_run_id": None,
                },
                csrf,
            )
            if edit_after_queue is not None:
                edit_after_queue(owner, csrf, app)
        results = []
        # Restart with a queued job: dispatch must use the persisted manifest.
        with server(env):
            owner = client()
            session = google_sign_in(owner, key, credentials)
            for scenario, expected in [
                ("pass", "passed"),
                ("fail", "failed"),
                ("blocked", "blocked"),
            ]:
                run = (
                    queued
                    if scenario == "pass"
                    else submit(
                        owner,
                        app,
                        {
                            "build_id": build["id"],
                            "source": {"kind": "release_plan", "plan_version_id": plan},
                            "environment_revision": 1,
                            "baseline_run_id": None,
                        },
                        session["csrf_token"],
                    )
                )
                state = ROOT / ".private/execution-smoke" / str(uuid.uuid4())
                subprocess.run(
                    [
                        "uv",
                        "run",
                        "--no-sync",
                        "--project",
                        str(ROOT / "apps/mobile-worker"),
                        "--frozen",
                        "mobile-qa-worker",
                        "execution-worker",
                        "--origin",
                        BASE,
                        "--profile-id",
                        profile,
                        "--state",
                        str(state),
                        "--scenario",
                        scenario,
                        "--once",
                    ],
                    env=env,
                    check=True,
                    timeout=120,
                )
                report = request(owner, "GET", "/api/runs/" + run["id"])
                assert report["manifest"] == queued["manifest"], report
                assert report["state"] == "finished", report
                assert report["attempts"][0]["outcome"] == expected, report
                assert report["attempts"][0]["cleanup"] == "verified_clean"
                assert len(report["attempts"][0]["artifacts"]) == 2 * len(
                    report["manifest"]["cases"][0]["case"]["actions"]
                )
                assert report["manifest"]["profile"]["driver"] == "fake"
                results.append(report)
        with server(env):
            owner = client()
            google_sign_in(owner, key, credentials)
            for report in results:
                assert request(owner, "GET", "/api/runs/" + report["id"]) == report
        print(
            "Execution HTTP smoke passed: queued-job restart, Python protocol, passed/failed/blocked evidence and reports after restart; simulated only"
        )


if __name__ == "__main__":
    main()
