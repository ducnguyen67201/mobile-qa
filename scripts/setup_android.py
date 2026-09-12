"""Explicit project-local Android intake tool installation, never run at API startup.

Pins are from Google's repository2-3.xml. Downloads go only to ignored .private;
no global SDK configuration, emulator, platform-tools or license state is changed.
"""

import hashlib
import os
import platform
import shutil
import tempfile
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PACKAGES = {
    "Darwin": (
        "build-tools_r36_macosx.zip",
        "199ae0047ee61e842f8ee0c6d3918e44fb9a1f83",
    ),
    "Linux": ("build-tools_r36_linux.zip", "b0b6376977657e8ad9b969bacf4093601da2c6fb"),
}


def install(archive_name, checksum, destination):
    if destination.exists():
        print(f"Already present: {destination}")
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=destination.parent) as tmp:
        archive = Path(tmp) / "download.zip"
        urllib.request.urlretrieve(
            "https://dl.google.com/android/repository/" + archive_name, archive
        )
        if hashlib.sha1(archive.read_bytes()).hexdigest() != checksum:
            raise RuntimeError("Google SDK archive checksum mismatch")
        unpacked = Path(tmp) / "unpacked"
        with zipfile.ZipFile(archive) as source:
            source.extractall(unpacked)
            for item in source.infolist():
                mode = item.external_attr >> 16
                if mode and (unpacked / item.filename).is_file():
                    (unpacked / item.filename).chmod(mode & 0o777)
        folders = list(unpacked.iterdir())
        if len(folders) != 1:
            raise RuntimeError("Unexpected SDK package layout")
        shutil.move(str(folders[0]), destination)
        print(
            f"Installed {archive_name}; SHA-256 {hashlib.sha256(archive.read_bytes()).hexdigest()}"
        )


def main():
    os.umask(0o077)
    sdk = ROOT / ".private/android-sdk"
    install(*PACKAGES[platform.system()], sdk / "build-tools/36.0.0")
    install(
        "platform-35_r02.zip",
        "0bb560a90a7a2cbd0dd8348224d518b638fe7949",
        sdk / "platforms/android-35",
    )


if __name__ == "__main__":
    main()
