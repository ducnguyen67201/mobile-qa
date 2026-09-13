"""Create signed synthetic intake fixtures with explicitly installed Android/JDK tools."""

import hashlib
import json
import os
import secrets
import shutil
import subprocess
import tempfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main():
    os.umask(0o077)
    sdk = Path(os.environ.get("MOBILE_QA_ANDROID_SDK", ROOT / ".private/android-sdk"))
    tools = sdk / "build-tools/36.0.0"
    java = Path(os.environ["JAVA_HOME"]) / "bin"
    if not (tools / "aapt2").is_file() or not (java / "keytool").is_file():
        raise RuntimeError(
            "Run scripts/setup_android.py and set JAVA_HOME to JDK 17 before APK tests"
        )
    output = ROOT / ".private/test-apks"
    output.mkdir(parents=True, exist_ok=True)
    source = (ROOT / "apps/api/tests/fixtures/apk/AndroidManifest.xml").read_text()
    env = {
        "PATH": str(java) + ":/usr/bin:/bin",
        "JAVA_HOME": str(java.parent),
        "LANG": "C",
        "FIXTURE_PASSWORD": secrets.token_urlsafe(32),
    }
    expected = {}
    with tempfile.TemporaryDirectory(dir=output) as tmp:
        temp = Path(tmp)
        key = temp / "synthetic.p12"

        def run(command):
            subprocess.run(
                [str(x) for x in command],
                env=env,
                check=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
                timeout=60,
            )

        run(
            [
                java / "keytool",
                "-genkeypair",
                "-keystore",
                key,
                "-storepass:env",
                "FIXTURE_PASSWORD",
                "-keypass:env",
                "FIXTURE_PASSWORD",
                "-alias",
                "fixture",
                "-keyalg",
                "RSA",
                "-keysize",
                "2048",
                "-validity",
                "2",
                "-dname",
                "CN=Synthetic Intake Test",
            ]
        )
        for name, manifest in {
            "valid": source,
            "execution": source.replace("com.mobileqa.fixture", "ai.mobileqa.demo"),
            "mismatch": source.replace("com.mobileqa.fixture", "com.mobileqa.other"),
            "unsupported": source.replace('minSdkVersion="23"', 'minSdkVersion="36"'),
        }.items():
            xml = temp / "AndroidManifest.xml"
            xml.write_text(manifest)
            unsigned, aligned = temp / "unsigned.apk", temp / "aligned.apk"
            run(
                [
                    tools / "aapt2",
                    "link",
                    "--manifest",
                    xml,
                    "-I",
                    sdk / "platforms/android-35/android.jar",
                    "-o",
                    unsigned,
                ]
            )
            run([tools / "zipalign", "-f", "4", unsigned, aligned])
            run(
                [
                    tools / "apksigner",
                    "sign",
                    "--ks",
                    key,
                    "--ks-key-alias",
                    "fixture",
                    "--ks-pass",
                    "env:FIXTURE_PASSWORD",
                    "--key-pass",
                    "env:FIXTURE_PASSWORD",
                    "--out",
                    output / f"{name}.apk",
                    aligned,
                ]
            )
            if name == "valid":
                shutil.copyfile(unsigned, output / "unsigned.apk")
                shutil.copyfile(output / "valid.apk", output / "tampered.apk")
                with zipfile.ZipFile(output / "tampered.apk", "a") as tampered:
                    tampered.writestr("tampered.txt", "changed after signing")
        (output / "corrupt.apk").write_bytes(b"This is not an APK")
        for apk in output.glob("*.apk"):
            data = apk.read_bytes()
            expected[apk.stem] = {
                "size": len(data),
                "sha256": hashlib.sha256(data).hexdigest(),
            }
    expected["valid"].update(
        package_name="com.mobileqa.fixture",
        version_name="1.2 fixture",
        version_code="7",
        min_sdk=23,
        target_sdk=35,
    )
    (output / "expected.json").write_text(json.dumps(expected, indent=2) + "\n")
    print(
        "Synthetic APK fixtures prepared; no customer binaries or signing keys retained"
    )


if __name__ == "__main__":
    main()
