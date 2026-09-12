"""Exercise the pinned installer with local archives only; never contact Google."""

import hashlib
import importlib.util
import json
import zipfile
from xml.etree import ElementTree

import pytest

from mobile_qa_worker.qualification.runner import ROOT


def installer():
    path = ROOT / "infra/device-host/install_tools.py"
    spec = importlib.util.spec_from_file_location("qualification_installer", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_lock_metadata_is_namespaced_and_pinned():
    lock = json.loads((ROOT / "infra/device-host/toolchain.lock.json").read_text())
    for package in lock["packages"]:
        root = ElementTree.fromstring(package["package_xml"])
        assert root.tag.startswith("{http://schemas.android.com/")
        local = root.find("localPackage")
        assert local is not None and local.attrib["path"] == package["path"]
        assert root.find("license") is not None
        for archive in package["archives"]:
            assert archive["url"].startswith("https://dl.google.com/android/repository/")
            assert len(archive["sha1"]) == 40


@pytest.mark.parametrize("case", ["valid", "checksum", "traversal"])
def test_install_checks_archive_and_preserves_managed_metadata(tmp_path, monkeypatch, case):
    module = installer()
    archive = tmp_path / "archive.zip"
    with zipfile.ZipFile(archive, "w") as zipped:
        zipped.writestr("../escape" if case == "traversal" else "platform-tools/adb", b"tool")
    digest = hashlib.sha1(archive.read_bytes()).hexdigest()
    lock = tmp_path / "lock.json"
    package = {
        "path": "platform-tools",
        "package_xml": "<metadata/>",
        "archives": [{"host": "all", "arch": "all", "url": "local-test", "sha1": digest}],
    }
    if case == "checksum":
        package["archives"][0]["sha1"] = "0" * 40
    lock.write_text(json.dumps({"packages": [package]}))
    monkeypatch.setattr(module.urllib.request, "urlopen", lambda *a, **k: archive.open("rb"))
    root = tmp_path / "sdk"
    if case != "valid":
        with pytest.raises(ValueError):
            module.install(lock, root, demo_only=True)
        assert not (root / "platform-tools").exists()
        return
    module.install(lock, root, demo_only=True)
    assert (root / "platform-tools/adb").read_bytes() == b"tool"
    (root / "platform-tools/package.xml").write_text("stale")
    module.install(lock, root, demo_only=True)
    assert (root / "platform-tools/package.xml").read_text() == "<metadata/>"
    (root / "platform-tools/.mobile-qa-archive-sha1").unlink()
    with pytest.raises(ValueError, match="Refusing to overwrite"):
        module.install(lock, root, demo_only=True)
