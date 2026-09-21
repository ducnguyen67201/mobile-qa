"""Explicit local SDK setup and demo build; never runs checks or accepts SDK licenses."""

import argparse
import json
import os
import platform
import shlex
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SDK = ROOT / ".private/android-sdk"
LOCAL = ROOT / ".private/device-local"


def java_environment() -> dict[str, str]:
    env = dict(os.environ)
    if not env.get("JAVA_HOME") and platform.system() == "Darwin":
        found = subprocess.run(
            ["/usr/libexec/java_home", "-v", "17"], capture_output=True, text=True
        )
        if found.returncode == 0:
            env["JAVA_HOME"] = found.stdout.strip()
        else:
            homebrew = Path(
                "/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home"
            )
            if homebrew.is_dir():
                env["JAVA_HOME"] = str(homebrew.resolve())
    # Select one SDK even when a host exports the deprecated location alias.
    env.pop("ANDROID_SDK_ROOT", None)
    env["ANDROID_HOME"] = str(SDK)
    return env


def setup() -> None:
    system, machine = platform.system(), platform.machine().lower()
    if system == "Darwin" and machine in ("arm64", "aarch64"):
        abi = "arm64-v8a"
    elif system in ("Darwin", "Linux") and machine in ("x86_64", "amd64"):
        abi = "x86_64"
    else:
        raise SystemExit("Unsupported host. Use native macOS or Linux x86_64.")
    LOCAL.mkdir(parents=True, mode=0o700, exist_ok=True)
    profile = LOCAL / "profile.toml"
    if not profile.exists():
        values = {
            "sdk_root": str(SDK),
            "state_root": str(LOCAL / "state"),
            "toolchain": str(ROOT / "infra/device-host/toolchain.lock.json"),
            "system_image": f"system-images;android-35;google_apis;{abi}",
            "headless": system != "Darwin",
            "doppler_project": "mobile-qa",
            "doppler_config": "dev",
        }
        with profile.open("x") as stream:
            profile.chmod(0o600)
            stream.write(
                "# Nonsecret local device settings. API/cloud configuration is separate.\n"
            )
            stream.writelines(f"{k} = {json.dumps(v)}\n" for k, v in values.items())
    subprocess.run(
        [
            "python3",
            str(ROOT / "infra/device-host/install_tools.py"),
            "--sdk-root",
            str(SDK),
        ],
        check=True,
    )
    env = java_environment()
    sdkmanager = SDK / "cmdline-tools/19.0/bin/sdkmanager"
    print("SDK downloaded. Review and accept its licenses interactively before use:")
    prefix = (
        f"JAVA_HOME={shlex.quote(env['JAVA_HOME'])} " if env.get("JAVA_HOME") else ""
    )
    print(prefix + shlex.join([str(sdkmanager), f"--sdk_root={SDK}", "--licenses"]))
    print("Then: just device-local-build\n      just device-local")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["setup", "build", "run"])
    parser.add_argument(
        "--scenario", choices=["good", "broken", "unavailable"], default="good"
    )
    parser.add_argument("--resolved-model", type=Path)
    args = parser.parse_args()
    if args.action == "setup":
        setup()
    elif args.action == "build":
        subprocess.run(
            [
                "./gradlew",
                "--no-daemon",
                ":app:assembleGoodDebug",
                ":app:assembleBrokenDebug",
            ],
            cwd=ROOT / "apps/qa-demo-android",
            env=java_environment(),
            check=True,
        )
    else:
        command = [
            str(ROOT / "apps/mobile-worker/.venv/bin/python"),
            "-m",
            "mobile_qa_worker.cli",
            "device-local",
            "--profile",
            str(LOCAL / "profile.toml"),
            "--scenario",
            args.scenario,
        ]
        if args.resolved_model:
            command.extend(
                ["--agent", "--resolved-model", str(args.resolved_model.resolve())]
            )
        # Replace the wrapper so Ctrl-C reaches the supervisor without an outer
        # subprocess.call killing it while its bounded cleanup is still running.
        os.chdir(ROOT)
        os.execve(command[0], command, java_environment())


if __name__ == "__main__":
    main()
