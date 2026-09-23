"""Private, bounded APK cache; filesystem locks also fence concurrent processes.

App UUIDs are globally unique and belong to one organization. Including the app in
our cache key prevents cross-tenant reuse even when two builds have the same hash.
Only complete verified files can be pinned. Android receives a read-only source
file; writable guest userdata is never linked into this cache.
"""

import fcntl
import hashlib
import os
import re
import shutil
import stat
import time
from collections.abc import Callable, Iterator
from contextlib import contextmanager
from pathlib import Path
from typing import IO
from uuid import UUID

MAX_APK_BYTES = 2 * 1024**3
CHUNK_BYTES = 64 * 1024
CACHE_BYTES = 50 * 1024**3
SAFETY_BYTES = 15 * 1024**3
Check = Callable[[], None]
Loader = Callable[[Path], None]


class ArtifactError(ValueError):
    """Safe reason codes only; paths and signed capabilities never reach errors."""


def _continue() -> None:
    pass


def _private_directory(path: Path) -> None:
    if not path.is_absolute() or path != path.resolve():
        raise ArtifactError("unsafe_cache_root")
    path.mkdir(mode=0o700, parents=True, exist_ok=True)
    if not path.is_dir() or path.is_symlink():
        raise ArtifactError("unsafe_cache_root")
    path.chmod(0o700)


def _lock_file(path: Path) -> IO[bytes]:
    descriptor = os.open(path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    if not stat.S_ISREG(os.fstat(descriptor).st_mode):
        os.close(descriptor)
        raise ArtifactError("unsafe_cache_lock")
    return os.fdopen(descriptor, "r+b")


def _lock(handle: IO[bytes], mode: int, check: Check) -> None:
    while True:
        check()
        try:
            fcntl.flock(handle, mode | fcntl.LOCK_NB)
            return
        except BlockingIOError:
            time.sleep(0.1)


def _regular(path: Path) -> bool:
    try:
        return stat.S_ISREG(path.lstat().st_mode)
    except FileNotFoundError:
        return False


def _digest(path: Path, check: Check) -> str:
    digest = hashlib.sha256()
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(descriptor, "rb") as source:
        while True:
            check()
            chunk = source.read(CHUNK_BYTES)
            if not chunk:
                return digest.hexdigest()
            digest.update(chunk)


class Cache:
    def __init__(
        self, root: Path, *, max_bytes: int = CACHE_BYTES, safety_bytes: int = SAFETY_BYTES
    ) -> None:
        if max_bytes < 1 or safety_bytes < 0:
            raise ArtifactError("invalid_cache_policy")
        _private_directory(root)
        self.root = root
        self.max_bytes = max_bytes
        self.safety_bytes = safety_bytes

    @classmethod
    def from_environment(cls, state: Path) -> "Cache":
        # A host service supplies a common root for all execution and phone slots.
        root = Path(os.environ.get("MOBILE_QA_APK_CACHE_ROOT", str(state.parent / "apk-cache")))
        return cls(root)

    def admit(self, required_bytes: int = 0, check: Check = _continue) -> None:
        """Reject claims/preparation when active transfers consume reserved headroom."""
        if required_bytes < 0:
            raise ArtifactError("invalid_disk_reservation")
        with _lock_file(self.root / "index.lock") as handle:
            _lock(handle, fcntl.LOCK_EX, check)
            self._make_room(required_bytes, check)

    def resident_bytes(self) -> int:
        """Report completed cache bytes without following unrelated paths or links."""
        with _lock_file(self.root / "index.lock") as handle:
            _lock(handle, fcntl.LOCK_SH, _continue)
            return sum(path.stat().st_size for path in self._files(".apk"))

    def _files(self, suffix: str) -> list[Path]:
        paths: list[Path] = []
        for directory in self.root.iterdir():
            if directory.is_symlink() or not directory.is_dir():
                continue
            try:
                UUID(directory.name)
            except ValueError:
                continue
            paths.extend(p for p in directory.glob("*" + suffix) if _regular(p))
        return paths

    def _reservations(self) -> tuple[int, int]:
        reserved = remaining = 0
        for marker in self._files(".reserve"):
            # Markers are bounded local bookkeeping, never externally supplied JSON.
            if marker.stat().st_size > 20:
                raise ArtifactError("invalid_cache_reservation")
            size = int(marker.read_text())
            if not 0 < size <= MAX_APK_BYTES:
                raise ArtifactError("invalid_cache_reservation")
            partial = marker.with_suffix(".partial")
            # The HTTP client can discard its failed partial while this scan runs.
            # Take a single metadata snapshot; the reservation remains authoritative.
            try:
                metadata = partial.lstat()
                written = metadata.st_size if stat.S_ISREG(metadata.st_mode) else 0
            except FileNotFoundError:
                written = 0
            reserved += size
            remaining += max(0, size - written)
        return reserved, remaining

    def _reap_partials(self) -> None:
        # An exclusive entry lock proves no downloader can still own its scratch.
        for marker in self._files(".reserve"):
            with _lock_file(marker.with_suffix(".lock")) as handle:
                try:
                    fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError:
                    continue
                marker.with_suffix(".partial").unlink(missing_ok=True)
                marker.unlink(missing_ok=True)

    def _make_room(self, size: int, check: Check) -> None:
        self._reap_partials()
        reserved, remaining = self._reservations()
        entries = sorted(self._files(".apk"), key=lambda p: p.stat().st_mtime_ns)
        used = sum(p.stat().st_size for p in entries)

        def enough() -> bool:
            return (
                used + reserved + size <= self.max_bytes
                and shutil.disk_usage(self.root).free >= self.safety_bytes + remaining + size
            )

        for path in entries:
            if enough():
                return
            check()
            with _lock_file(path.with_suffix(".lock")) as handle:
                try:
                    fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError:
                    continue
                # A crash can leave a consumer's read-only link; retain it until recovery.
                if path.stat().st_nlink != 1:
                    continue
                used -= path.stat().st_size
                path.unlink()
        if not enough():
            raise ArtifactError("artifact_disk_capacity_exhausted")

    def _valid(self, path: Path, size: int, digest: str, check: Check) -> bool:
        return (
            _regular(path)
            and path.stat().st_size == size
            and path.stat().st_mode & 0o222 == 0
            and _digest(path, check) == digest
        )

    @contextmanager
    def acquire(
        self,
        app_id: UUID,
        digest: str,
        size: int,
        load: Loader,
        check: Check = _continue,
    ) -> Iterator[Path]:
        if not re.fullmatch(r"[0-9a-f]{64}", digest) or not 0 < size <= MAX_APK_BYTES:
            raise ArtifactError("invalid_artifact_identity")
        scope = self.root / str(UUID(str(app_id)))
        _private_directory(scope)
        path = scope / (digest + ".apk")
        with _lock_file(path.with_suffix(".lock")) as handle:
            _lock(handle, fcntl.LOCK_SH, check)
            if not self._valid(path, size, digest, check):
                fcntl.flock(handle, fcntl.LOCK_UN)
                _lock(handle, fcntl.LOCK_EX, check)
                if not self._valid(path, size, digest, check):
                    if path.exists() or path.is_symlink():
                        if path.is_symlink() or path.stat().st_nlink != 1:
                            raise ArtifactError("unsafe_or_pinned_cache_entry")
                        with _lock_file(self.root / "index.lock") as index:
                            _lock(index, fcntl.LOCK_EX, check)
                            path.unlink()
                    self._populate(path, size, digest, load, check)
                fcntl.flock(handle, fcntl.LOCK_SH)
            check()
            os.utime(path, None, follow_symlinks=False)
            yield path

    def _populate(self, path: Path, size: int, digest: str, load: Loader, check: Check) -> None:
        partial, reservation = path.with_suffix(".partial"), path.with_suffix(".reserve")
        with _lock_file(self.root / "index.lock") as handle:
            _lock(handle, fcntl.LOCK_EX, check)
            partial.unlink(missing_ok=True)
            reservation.unlink(missing_ok=True)
            self._make_room(size, check)
            descriptor = os.open(reservation, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
            with os.fdopen(descriptor, "w") as marker:
                marker.write(str(size))
                marker.flush()
                os.fsync(marker.fileno())
        try:
            load(partial)
            if not _regular(partial) or partial.stat().st_size != size:
                raise ArtifactError("build_size_mismatch")
            if _digest(partial, check) != digest:
                raise ArtifactError("build_checksum_mismatch")
            partial.chmod(0o400)
            with partial.open("rb") as source:
                os.fsync(source.fileno())
            check()
            # Publication and reservation release are one index transition. Another
            # slot must never read half of a completion while reserving its disk.
            with _lock_file(self.root / "index.lock") as index:
                _lock(index, fcntl.LOCK_EX, check)
                os.replace(partial, path)
                reservation.unlink()
                descriptor = os.open(path.parent, os.O_RDONLY)
                try:
                    os.fsync(descriptor)
                finally:
                    os.close(descriptor)
        finally:
            # A failed/canceled stream must never become a reusable artifact.
            with _lock_file(self.root / "index.lock") as index:
                _lock(index, fcntl.LOCK_EX, _continue)
                partial.unlink(missing_ok=True)
                reservation.unlink(missing_ok=True)


@contextmanager
def installed_source(cached: Path, target: Path) -> Iterator[Path]:
    """Pin immutable install bytes without a second multi-GiB copy per emulator."""
    if not _regular(cached) or cached.stat().st_mode & 0o222:
        raise ArtifactError("unsafe_cache_source")
    if target.exists() or target.is_symlink():
        raise ArtifactError("artifact_target_exists")
    try:
        os.link(cached, target, follow_symlinks=False)
    except OSError:
        raise ArtifactError("artifact_link_failed_same_filesystem_required") from None
    try:
        yield target
    finally:
        target.unlink(missing_ok=True)
