"""Secret-free HTTP acceptance using owned test accounts/processes and a synthetic APK.

This is API integration evidence, not rendered browser acceptance or device evidence.
No Doppler, customer account, cloud artifact or database reset is involved.
"""
import hashlib
import http.cookiejar
import json
import os
import re
import secrets
import signal
import socket
import subprocess
import time
import urllib.error
import urllib.request
import uuid
from contextlib import contextmanager
from pathlib import Path
from runtime import ROOT, database, wait_http

ORIGIN = "http://127.0.0.1:5173"
BASE = "http://127.0.0.1:5151"
BINARY = ROOT / "target/debug/mobile-qa-cli"


def client():
    return urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))


def request(opener, method, path, data=None, csrf=None, content_type="application/json"):
    headers = {"Origin": ORIGIN, "X-Mobile-QA-Request": "1"}
    if csrf:
        headers["X-CSRF-Token"] = csrf
    if data is not None:
        if not isinstance(data, bytes):
            data = json.dumps(data).encode()
        headers["Content-Type"] = content_type
    req = urllib.request.Request(BASE + path, data=data, headers=headers, method=method)
    with opener.open(req, timeout=180) as response:
        assert response.headers["Cache-Control"] == "no-store"
        return json.load(response)


def provision(env):
    email = f"{uuid.uuid4()}@smoke.invalid"
    password = secrets.token_urlsafe(32)
    result = subprocess.run([str(BINARY), "task", "operator", "action:provision", f"email:{email}", "name:Synthetic smoke", "organization:Synthetic smoke", "--environment", "test"], cwd=ROOT / "apps/api", env={**env, "MOBILE_QA_OPERATOR_PASSWORD": password}, capture_output=True, text=True, check=True, timeout=30)
    match = re.search(r"user_id=([0-9a-f-]+) organization_id=([0-9a-f-]+)", result.stdout)
    if not match:
        raise RuntimeError("Provisioning did not return account identifiers")
    return {"email": email, "password": password}, match[2]


@contextmanager
def server(env):
    with socket.socket() as probe:
        # Allow our previous listener's TIME_WAIT sockets, but never steal a live port.
        probe.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        probe.bind(("127.0.0.1", 5151))
    log = ROOT / ".private/app-setup-smoke.log"
    with log.open("w") as handle:
        child = subprocess.Popen([str(BINARY), "start", "--environment", "test", "--port", "5151"], cwd=ROOT / "apps/api", env=env, stdout=handle, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            wait_http(BASE + "/api/health", [child])
            yield
        finally:
            if child.poll() is None:
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()


def main():
    os.umask(0o077)
    env = {**os.environ, "MOBILE_QA_TEST_SCOPE": str(uuid.uuid4())}
    with database():
        credentials, org = provision(env)
        other, _ = provision(env)
        with server(env):
            owner = client()
            session = request(owner, "POST", "/api/auth/login", credentials)
            csrf = session["csrf_token"]
            app = request(owner, "POST", "/api/apps", {"organization_id": org, "name": "Synthetic intake smoke", "android_package": "com.mobileqa.fixture", "environment_name": "Staging", "backend_origins": ["https://staging.smoke.invalid"], "login_origins": []}, csrf)
            base = f"/api/apps/{app['id']}"
            data = (ROOT / ".private/test-apks/valid.apk").read_bytes()
            upload = request(owner, "POST", base + "/build-uploads", {"original_filename": "synthetic.apk", "expected_size": len(data)}, csrf)
            upload_path = base + "/build-uploads/" + upload["id"]
            boundary = "mobileqa" + uuid.uuid4().hex
            body = f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="synthetic.apk"\r\nContent-Type: application/octet-stream\r\n\r\n'.encode() + data + f"\r\n--{boundary}--\r\n".encode()
            request(owner, "PUT", upload_path + "/content", body, csrf, "multipart/form-data; boundary=" + boundary)
            started = time.monotonic()
            build = request(owner, "POST", upload_path + "/complete", csrf=csrf)
            elapsed = time.monotonic() - started
            assert build["validation"]["state"] == "validated", build["validation"]
            assert build["sha256"] == hashlib.sha256(data).hexdigest()
            assert build["byte_size"] == len(data)
            assert build["metadata"]["package_name"] == "com.mobileqa.fixture"
            assert build["readiness"]["install"] == "not_checked" and not build["readiness"]["execution_ready"]
            assert request(owner, "POST", upload_path + "/complete", csrf=csrf)["id"] == build["id"]
            request(owner, "POST", "/api/auth/logout", csrf=csrf)
            foreign = client()
            request(foreign, "POST", "/api/auth/login", other)
            try:
                request(foreign, "GET", base + "/builds/" + build["id"])
                raise AssertionError("Cross-organization build leaked")
            except urllib.error.HTTPError as error:
                assert error.code == 404
        # Restart the API, re-run migrations non-destructively, and get the persisted build.
        with server(env):
            fresh = client()
            request(fresh, "POST", "/api/auth/login", credentials)
            restored = request(fresh, "GET", base + "/builds/" + build["id"])
            assert restored == build
            assert request(fresh, "GET", base + "/builds")["items"][0]["id"] == build["id"]
        print(f"App setup HTTP smoke passed: persisted build after API restart, tenant denial, finalization {elapsed:.3f}s")


if __name__ == "__main__":
    main()
