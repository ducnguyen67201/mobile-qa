"""Explicit real-device 07B acceptance. Never invoked by ordinary checks.

Run with the worker's uv environment, --profile pointing at a nonsecret host TOML,
and --good/--broken at the local-only sample/sampleBroken APKs. Uses isolated test
accounts/API and owns disposable AVDs; no customer credentials or model calls.
"""

import argparse
import hashlib
import json
import os
import secrets
import subprocess
import urllib.error
import urllib.request
import uuid
from pathlib import Path

from app_setup_smoke import (
    BASE,
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
from execution_smoke import import_file, task


def expected(name, resource, value, checkpoint, presence=False):
    return {
        "id": name,
        "checkpoint_id": checkpoint,
        "description": name,
        "method": "ui_element_presence_v1" if presence else "ui_property_equals_v1",
        "resource_id": "ai.mobileqa.demo:id/" + resource,
        "text_filter": "",
        "property": "text",
        "expected": value,
        "ready_resource_id": "ai.mobileqa.demo:id/ready_marker",
        "prerequisite_check_ids": [],
        "required": True,
        "observation_seconds": 3,
    }


def command(operation, target=None, text=None):
    body = {"operation": operation}
    if target:
        body["target"] = {"by": "resource_id", "value": "ai.mobileqa.demo:id/" + target}
    if text is not None:
        body["text"] = text
    return body


def qualify(host_file, good, directory):
    from mobile_qa_worker.automation.direct import check, execute
    from mobile_qa_worker.device.android import AndroidDevice, doctor
    from mobile_qa_worker.generated.models import DirectCommand, ExpectedCheck
    from mobile_qa_worker.qualification.config import Profile
    from mobile_qa_worker.qualification.evidence import Evidence, sha256
    from mobile_qa_worker.qualification.process import host_lock

    host = Profile.load(host_file)
    doctor(host)
    with host_lock(host.state_root) as dirty:
        if dirty.exists():
            raise RuntimeError("Owned host needs recovery before qualification")
        for iteration in range(2):
            device = AndroidDevice(
                host,
                Evidence(directory / f"qualification-{iteration}"),
                "ai.mobileqa.demo",
                "ai.mobileqa.demo/.MainActivity",
            )
            try:
                device.boot()
                device.install(good, sha256(good))
                device.launch()
                check(
                    device,
                    device.package,
                    ExpectedCheck.model_validate(
                        expected("empty", "task_row", "false", "preflight", True)
                    ),
                )
                device.capture_checkpoint("clean-start")
                if iteration == 0:
                    for body in [
                        command("set_text", "task_input", "Qualification proof"),
                        command("tap", "save_task"),
                        command("restart"),
                    ]:
                        execute(
                            device, device.package, DirectCommand.model_validate(body)
                        )
                    check(
                        device,
                        device.package,
                        ExpectedCheck.model_validate(
                            expected(
                                "persisted", "task_row", "Qualification proof", "saved"
                            )
                        ),
                    )
                    device.capture_checkpoint("persisted")
            finally:
                device.stop()
                device.discard()
    print(
        "Qualified local fixture: persisted on first AVD, absent on fresh second AVD",
        flush=True,
    )
    return host


def upload(owner, csrf, app, apk):
    data = apk.read_bytes()
    receipt = request(
        owner,
        "POST",
        f"/api/apps/{app}/build-uploads",
        {"original_filename": apk.name, "expected_size": len(data)},
        csrf,
    )
    path = f"/api/apps/{app}/build-uploads/{receipt['id']}"
    boundary = "regression" + uuid.uuid4().hex
    content = (
        f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="{apk.name}"\r\nContent-Type: application/octet-stream\r\n\r\n'.encode()
        + data
        + f"\r\n--{boundary}--\r\n".encode()
    )
    request(
        owner,
        "PUT",
        path + "/content",
        content,
        csrf,
        "multipart/form-data; boundary=" + boundary,
    )
    build = request(owner, "POST", path + "/complete", csrf=csrf)
    assert build["validation"]["state"] == "validated", build
    return build["id"]


def submit(owner, csrf, app, body):
    req = urllib.request.Request(
        BASE + f"/api/apps/{app}/case-runs",
        json.dumps(body).encode(),
        {
            "Origin": ORIGIN,
            "Content-Type": "application/json",
            "X-CSRF-Token": csrf,
            "Idempotency-Key": str(uuid.uuid4()),
        },
        method="POST",
    )
    with owner.open(req, timeout=30) as response:
        assert response.status == 201
        return json.load(response)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--good", type=Path, required=True)
    parser.add_argument("--broken", type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)
    directory = ROOT / ".private/regression-acceptance" / str(uuid.uuid4())
    directory.mkdir(parents=True)
    # Own a distinct state root; never dispose or reuse an interactive phone.
    host_text = args.profile.read_text()
    import re

    host_file = directory / "host.toml"
    host_file.write_text(
        re.sub(
            r"^state_root\s*=.*$",
            "state_root = " + json.dumps(str(directory / "device")),
            host_text,
            flags=re.M,
        )
    )
    host = qualify(host_file, args.good.resolve(), directory)
    env = {
        **os.environ,
        "MOBILE_QA_TEST_SCOPE": str(uuid.uuid4()),
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
            session = google_sign_in(owner, key, credentials)
            actor, csrf = session["user"]["id"], session["csrf_token"]
            app = request(
                owner,
                "POST",
                "/api/apps",
                {
                    "organization_id": org,
                    "name": "07B real local regression acceptance",
                    "android_package": "ai.mobileqa.demo",
                    "environment_name": "Local only fixture",
                    "backend_origins": ["https://synthetic.invalid"],
                    "login_origins": [],
                },
                csrf,
            )["id"]
            builds = [
                upload(owner, csrf, app, p.resolve()) for p in [args.good, args.broken]
            ]
            profile_id = str(uuid.uuid4())
            reference = (
                str(directory.relative_to(ROOT)) + "/qualification-1/clean-start.xml"
            )
            profile = {
                "id": profile_id,
                "name": "Qualified local Tasks fixture",
                "driver": "direct",
                "package": "ai.mobileqa.demo",
                "adapter": "android_direct_v1",
                "device_identity": "local-regression-fixture",
                "image": host.system_image,
                "qualified": True,
                "qualification_reference": reference,
                "max_apk_bytes": 104857600,
            }
            profile["execution_context"] = {
                "schema_version": 1,
                "adapter_revision": "android_direct_v1",
                "verifier_revision": "ui_v1",
                "worker_runtime_revision": "direct_v1",
                "reset_policy_hash": hashlib.sha256(b"fresh-owned-avd-v1").hexdigest(),
                "qualified_profile_id": profile_id,
                "package": "ai.mobileqa.demo",
                "launch_component": "ai.mobileqa.demo/.MainActivity",
                "image": host.system_image,
                "abi": host.abi,
                "width": 1080,
                "height": 1920,
                "density": 420,
                "locale": "en-US",
                "timezone": "Etc/UTC",
                "state_scope": "local_only",
                "qualification_reference": reference,
                "starting_checks": [
                    expected("empty", "task_row", "false", "preflight", True)
                ],
                "stages": {
                    "boot_seconds": 180,
                    "install_seconds": 90,
                    "start_seconds": 60,
                    "cleanup_seconds": 240,
                },
            }
            path = directory / "registered-profile.json"
            path.write_text(json.dumps(profile))
            task(env, actor, app, "register-profile", file=path)
            case = {
                "kind": "case",
                "content": {
                    "key": "saved-task-persists",
                    "version": 1,
                    "title": "A saved task survives an app restart",
                    "requirement": "Saved text survives restart",
                    "provenance": "operator_authored",
                    "package": "ai.mobileqa.demo",
                    "adapter": "android_direct_v1",
                    "preconditions": ["Fresh local app state"],
                    "actions": [
                        {
                            "id": "enter",
                            "kind": "direct",
                            "instruction": "",
                            "checkpoint_id": "entered",
                            "command": command("set_text", "task_input", "Buy milk"),
                        },
                        {
                            "id": "save",
                            "kind": "direct",
                            "instruction": "",
                            "checkpoint_id": "saved",
                            "command": command("tap", "save_task"),
                        },
                        {
                            "id": "restart",
                            "kind": "direct",
                            "instruction": "",
                            "checkpoint_id": "reopened",
                            "command": command("restart"),
                        },
                    ],
                    "checks": [
                        expected("entered", "task_input", "Buy milk", "entered"),
                        expected("saved", "task_row", "Buy milk", "saved"),
                        expected("persists", "task_row", "Buy milk", "reopened"),
                    ],
                    "budget": {
                        "duration_seconds": 300,
                        "max_steps": 30,
                        "artifact_bytes": 16777216,
                    },
                },
            }
            case_id = import_file(env, actor, app, directory, case)
            task(
                env,
                actor,
                app,
                "register-worker",
                worker=str(uuid.uuid4()),
                profile=profile_id,
            )
            results = []
            for index, (build, verdict) in enumerate(
                [
                    (builds[0], "no_baseline"),
                    (builds[1], "regression"),
                    (builds[1], "still_failing"),
                ]
            ):
                body = {
                    "case_version_id": case_id,
                    "build_id": build,
                    "profile_id": profile_id,
                    "environment_revision": 1,
                    "baseline_run_id": results[-1]["id"] if results else None,
                }
                preview = request(
                    owner, "POST", f"/api/apps/{app}/case-runs/preview", body, csrf
                )
                assert not preview["blockers"], preview
                if results:
                    assert preview["suggested_baseline_id"] == results[-1]["id"], (
                        preview
                    )
                run = submit(owner, csrf, app, body)
                print(f"Executing real run {index + 1}: {run['id']}", flush=True)
                with (directory / f"worker-{index}.log").open("w") as log:
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
                            profile_id,
                            "--state",
                            str(directory / f"worker-{index}"),
                            "--profile",
                            str(host_file),
                            "--once",
                        ],
                        env=env,
                        check=True,
                        timeout=900,
                        stdout=log,
                        stderr=subprocess.STDOUT,
                    )
                report = request(owner, "GET", "/api/runs/" + run["id"])
                (directory / f"run-{index}.json").write_text(
                    json.dumps(report, indent=2)
                )
                assert report["state"] == "finished", report
                assert report["comparison"]["cases"][0]["kind"] == verdict, report
                assert report["attempts"][0]["outcome"] == (
                    "passed" if index == 0 else "failed"
                ), report
                if index:
                    failed = report["comparison"]["cases"][0]["current_checks"][-1]
                    assert (
                        failed["check_id"] == "persists"
                        and failed["observation_kind"] == "absent"
                    ), failed
                results.append(report)
                print(f"Verified: {verdict}", flush=True)
        with server(env):
            owner = client()
            google_sign_in(owner, key, credentials)
            for report in results:
                assert request(owner, "GET", "/api/runs/" + report["id"]) == report
            history = request(owner, "GET", f"/api/apps/{app}/run-history")
            assert {i["run"]["id"] for i in history["items"]} == {
                r["id"] for r in results
            }
        print(
            f"Real pass → regression → still failing and restart/history passed. Evidence: {directory}"
        )


if __name__ == "__main__":
    try:
        main()
    except urllib.error.HTTPError as error:
        raise RuntimeError(error.read().decode()) from None
