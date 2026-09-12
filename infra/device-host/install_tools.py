"""Explicit pinned archive installation into a dedicated SDK root; never accepts licenses."""

import argparse
import hashlib
import json
from pathlib import Path
import platform
import shutil
import tempfile
import urllib.request
import zipfile


def install(lock_path: Path, root: Path, demo_only: bool = False) -> None:
    host = "macosx" if platform.system() == "Darwin" else "linux"
    arch = "aarch64" if platform.machine() in ("arm64", "aarch64") else "x64"
    root.mkdir(parents=True, exist_ok=True)
    for package in json.loads(lock_path.read_text())["packages"]:
        if demo_only and package["path"] not in (
            "platforms;android-35",
            "build-tools;35.0.0",
            "cmdline-tools;19.0",
            "platform-tools",
        ):
            continue
        target = root / package["path"].replace(";", "/")
        marker = target / ".mobile-qa-archive-sha1"
        archive = next(
            a
            for a in package["archives"]
            if a["host"] in (host, "all") and a["arch"] in (arch, "all")
        )
        if marker.exists() and marker.read_text() == archive["sha1"]:
            (target / "package.xml").write_text(package["package_xml"])
            continue
        if target.exists():
            raise ValueError(
                "Refusing to overwrite unmanaged or mismatched SDK package: "
                + str(target)
            )
        with tempfile.TemporaryDirectory(prefix="mobile-qa-install-") as temp:
            folder = Path(temp)
            path = folder / "archive.zip"
            with (
                urllib.request.urlopen(archive["url"], timeout=60) as source,
                path.open("wb") as out,
            ):
                shutil.copyfileobj(source, out)
            with path.open("rb") as stream:
                if hashlib.file_digest(stream, "sha1").hexdigest() != archive["sha1"]:
                    raise ValueError("Official archive checksum mismatch")
            extracted = folder / "extracted"
            extracted.mkdir()
            with zipfile.ZipFile(path) as zipped:
                for member in zipped.infolist():
                    if (
                        not (extracted / member.filename)
                        .resolve()
                        .is_relative_to(extracted.resolve())
                    ):
                        raise ValueError("Unsafe archive path")
                zipped.extractall(extracted)
                for member in zipped.infolist():
                    mode = member.external_attr >> 16
                    if mode & 0o111:
                        (extracted / member.filename).chmod(0o755)
            children = list(extracted.iterdir())
            if len(children) != 1 or not children[0].is_dir():
                raise ValueError("Unexpected archive layout")
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.move(str(children[0]), target)
            (target / "package.xml").write_text(package["package_xml"])
            marker.write_text(archive["sha1"])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sdk-root", required=True, type=Path)
    parser.add_argument("--demo-only", action="store_true")
    args = parser.parse_args()
    install(
        Path(__file__).with_name("toolchain.lock.json"),
        args.sdk_root.resolve(),
        args.demo_only,
    )
