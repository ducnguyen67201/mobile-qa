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


def submit(opener, app, body, csrf, source="plan", key=None):
    req = urllib.request.Request(
        BASE + f"/api/apps/{app}/{'suite-runs' if source == 'suite' else 'runs'}",
        json.dumps(body).encode(),
        {
            "Origin": ORIGIN,
            "Content-Type": "application/json",
            "X-CSRF-Token": csrf,
            "Idempotency-Key": key or str(uuid.uuid4()),
        },
        method="POST",
    )
    with opener.open(req, timeout=30) as response:
        assert response.status in (200, 201)
        return json.load(response)


def main(author=None, edit_after_queue=None, source="plan"):
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
            if source == "plan":
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
                    owner,
                    "GET",
                    f"/api/apps/{app}/execution-plan?build_id={build['id']}",
                )
            else:
                preview = request(
                    owner,
                    "POST",
                    f"/api/apps/{app}/suite-runs/preview",
                    {
                        "suite_version_id": plan,
                        "build_id": build["id"],
                        "profile_id": profile,
                        "environment_revision": 1,
                    },
                    csrf,
                )
            assert not preview["blockers"]
            run_body = (
                {
                    "suite_version_id": plan,
                    "build_id": build["id"],
                    "profile_id": profile,
                    "environment_revision": 1,
                    "baseline_run_id": None,
                }
                if source == "suite"
                else {
                    "build_id": build["id"],
                    "plan_version_id": plan,
                    "environment_revision": 1,
                }
            )
            queued_key = str(uuid.uuid4())
            queued = submit(owner, app, run_body, csrf, source, queued_key)
            if source == "suite":
                assert submit(owner, app, run_body, csrf, source, queued_key) == queued
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
                        {**run_body, "baseline_run_id": results[0]["id"]}
                        if source == "suite"
                        else run_body,
                        session["csrf_token"],
                        source,
                    )
                )
                for _ in run["manifest"]["cases"]:
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
                assert len(report["attempts"]) == len(report["manifest"]["cases"])
                for index, attempt in enumerate(report["attempts"]):
                    assert attempt["outcome"] == expected, report
                    assert attempt["cleanup"] == "verified_clean"
                    assert len(attempt["artifacts"]) == 2 * len(
                        report["manifest"]["cases"][index]["case"]["actions"]
                    )
                if source == "suite" and scenario != "pass":
                    assert report["baseline_run_id"] == results[0]["id"]
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
