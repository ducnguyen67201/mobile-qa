"""Deterministic host storage tests: no cloud, Android, or multi-GiB allocation."""

import fcntl
import hashlib
import os
import threading
from concurrent.futures import ThreadPoolExecutor
from types import SimpleNamespace
from uuid import uuid4

import pytest

from mobile_qa_worker.artifacts.cache import ArtifactError, Cache, installed_source


def identity(data=b"apk"):
    return hashlib.sha256(data).hexdigest()


def cache(tmp_path, size=64):
    return Cache(tmp_path / "cache", max_bytes=size, safety_bytes=0)


def test_twenty_uses_have_one_verified_download(tmp_path):
    store, app, loaded = cache(tmp_path), uuid4(), []

    def load(path):
        loaded.append(path)
        path.write_bytes(b"apk")

    for _ in range(20):
        with store.acquire(app, identity(), 3, load) as path:
            assert path.read_bytes() == b"apk"
            assert path.stat().st_mode & 0o222 == 0
    assert len(loaded) == 1
    assert not list(store.root.rglob("*.partial"))
    assert not list(store.root.rglob("*.reserve"))


def test_same_hash_in_different_apps_never_reuses_tenant_bytes(tmp_path):
    store, loaded = cache(tmp_path), []

    def load(path):
        loaded.append(path)
        path.write_bytes(b"apk")

    paths = []
    for _ in range(2):
        with store.acquire(uuid4(), identity(), 3, load) as path:
            paths.append(path)
    assert len(loaded) == 2
    assert paths[0].parent != paths[1].parent


def test_two_concurrent_misses_have_only_one_writer(tmp_path):
    store, app = cache(tmp_path), uuid4()
    started, release = threading.Event(), threading.Event()
    loaded = []

    def load(path):
        loaded.append(path)
        started.set()
        assert release.wait(3)
        path.write_bytes(b"apk")

    def read():
        with store.acquire(app, identity(), 3, load) as path:
            return path.read_bytes()

    with ThreadPoolExecutor(max_workers=2) as executor:
        first = executor.submit(read)
        assert started.wait(3)
        second = executor.submit(read)
        release.set()
        assert first.result(timeout=3) == second.result(timeout=3) == b"apk"
    assert len(loaded) == 1


def test_lru_never_evicts_pinned_entry(tmp_path):
    store, app = cache(tmp_path, size=4), uuid4()
    with store.acquire(app, identity(), 3, lambda p: p.write_bytes(b"apk")) as original:
        with pytest.raises(ArtifactError, match="capacity_exhausted"):
            with store.acquire(app, identity(b"next"), 4, lambda p: p.write_bytes(b"next")):
                pytest.fail("evicted pinned bytes")
        assert original.exists()
    with store.acquire(app, identity(b"next"), 4, lambda p: p.write_bytes(b"next")) as other:
        assert other.read_bytes() == b"next"
        assert not original.exists()


def test_readonly_consumer_link_survives_crash_pin_until_recovery(tmp_path):
    store, app = cache(tmp_path, size=4), uuid4()
    with store.acquire(app, identity(), 3, lambda p: p.write_bytes(b"apk")) as path:
        pin = installed_source(path, tmp_path / "build.apk")
        target = pin.__enter__()
        assert os.stat(path).st_ino == os.stat(target).st_ino
        assert target.stat().st_mode & 0o222 == 0
    try:
        with pytest.raises(ArtifactError, match="capacity_exhausted"):
            with store.acquire(app, identity(b"next"), 4, lambda p: p.write_bytes(b"next")):
                pytest.fail("evicted crash-retained install source")
    finally:
        pin.__exit__(None, None, None)
    assert not target.exists()
    with store.acquire(app, identity(b"next"), 4, lambda p: p.write_bytes(b"next")):
        pass


def test_corrupt_entry_is_verified_and_replaced(tmp_path):
    store, app, calls = cache(tmp_path), uuid4(), []

    def load(path):
        calls.append(path)
        path.write_bytes(b"apk")

    with store.acquire(app, identity(), 3, load) as path:
        pass
    path.chmod(0o600)
    path.write_bytes(b"bad")
    path.chmod(0o400)
    with store.acquire(app, identity(), 3, load) as repaired:
        assert repaired.read_bytes() == b"apk"
    assert len(calls) == 2


def test_failed_download_and_bad_digest_leave_no_reusable_or_partial_file(tmp_path):
    store, app = cache(tmp_path), uuid4()
    with pytest.raises(ArtifactError, match="checksum_mismatch"):
        with store.acquire(app, identity(), 3, lambda p: p.write_bytes(b"bad")):
            pytest.fail("published corrupt artifact")
    assert not list(store.root.rglob("*.apk"))
    assert not list(store.root.rglob("*.partial"))
    assert not list(store.root.rglob("*.reserve"))

    def failed(path):
        path.write_bytes(b"a")
        raise OSError("interrupted")

    with pytest.raises(OSError):
        with store.acquire(app, identity(), 3, failed):
            pass
    assert not list(store.root.rglob("*.partial"))
    assert not list(store.root.rglob("*.reserve"))


def test_orphan_partial_is_reaped_after_process_lock_disappears(tmp_path):
    store = cache(tmp_path)
    scope = store.root / str(uuid4())
    scope.mkdir()
    (scope / (identity() + ".reserve")).write_text("3")
    (scope / (identity() + ".partial")).write_bytes(b"a")
    store.admit()
    assert not list(store.root.rglob("*.partial"))
    assert not list(store.root.rglob("*.reserve"))


def test_reservation_counts_full_download_before_bytes_arrive(tmp_path):
    store = cache(tmp_path, size=3)
    started, release = threading.Event(), threading.Event()

    def first():
        def load(path):
            started.set()
            assert release.wait(3)
            path.write_bytes(b"apk")

        with store.acquire(uuid4(), identity(), 3, load):
            pass

    with ThreadPoolExecutor(max_workers=1) as executor:
        future = executor.submit(first)
        assert started.wait(3)
        try:
            with pytest.raises(ArtifactError, match="capacity_exhausted"):
                with store.acquire(uuid4(), identity(), 3, lambda p: p.write_bytes(b"apk")):
                    pytest.fail("overbooked reserved disk")
        finally:
            release.set()
        future.result(timeout=3)


def test_disk_safety_reserve_is_checked_before_downloading(tmp_path, monkeypatch):
    store = Cache(tmp_path / "cache", max_bytes=100, safety_bytes=10)
    monkeypatch.setattr(
        "mobile_qa_worker.artifacts.cache.shutil.disk_usage", lambda _: SimpleNamespace(free=12)
    )
    with pytest.raises(ArtifactError, match="capacity_exhausted"):
        with store.acquire(uuid4(), identity(), 3, lambda _: pytest.fail("download started")):
            pass


def test_cache_rejects_symlink_and_invalid_digest(tmp_path):
    real = tmp_path / "real"
    real.mkdir()
    linked = tmp_path / "linked"
    linked.symlink_to(real)
    with pytest.raises(ArtifactError, match="unsafe_cache_root"):
        Cache(linked)
    store, app = cache(tmp_path), uuid4()
    with pytest.raises(ArtifactError, match="invalid_artifact_identity"):
        with store.acquire(app, "../private", 3, lambda _: None):
            pass
    scope = store.root / str(app)
    scope.mkdir()
    private = tmp_path / "private"
    private.write_bytes(b"apk")
    (scope / (identity() + ".apk")).symlink_to(private)
    with pytest.raises(ArtifactError, match="unsafe_or_pinned_cache_entry"):
        with store.acquire(app, identity(), 3, lambda _: None):
            pass
    assert private.read_bytes() == b"apk"


def test_cache_accepts_exact_two_gib_without_allocating_fixture(tmp_path, monkeypatch):
    store = Cache(tmp_path / "cache", max_bytes=2 * 1024**3, safety_bytes=0)
    monkeypatch.setattr(store, "_make_room", lambda *_: None)

    class ReachedLoader(Exception):
        pass

    def load(_):
        raise ReachedLoader

    with pytest.raises(ReachedLoader):
        with store.acquire(uuid4(), identity(), 2 * 1024**3, load):
            pass
    with pytest.raises(ArtifactError, match="invalid_artifact_identity"):
        with store.acquire(uuid4(), identity(), 2 * 1024**3 + 1, load):
            pass


def test_metrics_count_only_completed_regular_entries(tmp_path):
    store, app = cache(tmp_path), uuid4()
    with store.acquire(app, identity(), 3, lambda p: p.write_bytes(b"apk")):
        assert store.resident_bytes() == 3
    outside = tmp_path / "outside.apk"
    outside.write_bytes(b"private")
    (store.root / str(app) / "unsafe.apk").symlink_to(outside)
    (store.root / str(app) / "partial.partial").write_bytes(b"unfinished")
    assert store.resident_bytes() == 3


def test_index_scan_cannot_observe_half_published_artifact(tmp_path, monkeypatch):
    store, app = cache(tmp_path), uuid4()
    renamed, release = threading.Event(), threading.Event()
    original_replace = os.replace

    def pause_publication(source, target):
        original_replace(source, target)
        renamed.set()
        assert release.wait(3)

    monkeypatch.setattr("mobile_qa_worker.artifacts.cache.os.replace", pause_publication)

    def populate():
        with store.acquire(app, identity(), 3, lambda p: p.write_bytes(b"apk")):
            pass

    with ThreadPoolExecutor(max_workers=2) as executor:
        writer = executor.submit(populate)
        assert renamed.wait(3)
        scan = executor.submit(store.resident_bytes)
        try:
            # The writer holds the index across rename and reservation removal.
            with (store.root / "index.lock").open("rb") as index:
                with pytest.raises(BlockingIOError):
                    fcntl.flock(index, fcntl.LOCK_EX | fcntl.LOCK_NB)
        finally:
            release.set()
        writer.result(timeout=3)
        assert scan.result(timeout=3) == 3
    assert not list(store.root.rglob("*.reserve"))
