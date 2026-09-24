"""Bounded generated device messages on a mode-0600, same-UID Unix socket."""

import base64
import ctypes
import os
import socket
import stat
import struct
import sys
import threading
from collections.abc import Callable
from pathlib import Path

from mobile_qa_worker.generated.models import SlotDeviceRequest, SlotDeviceResponse
from mobile_qa_worker.qualification.config import QualificationError

MAX_REQUEST = 65536
MAX_RESPONSE = 24 * 1024 * 1024


def read_frame(connection: socket.socket, limit: int) -> bytes:
    header = _read(connection, 4)
    length = int.from_bytes(header, "big")
    if not 1 <= length <= limit:
        raise QualificationError("slot_message_too_large")
    return _read(connection, length)


def _read(connection: socket.socket, size: int) -> bytes:
    data = bytearray()
    while len(data) < size:
        chunk = connection.recv(min(65536, size - len(data)))
        if not chunk:
            raise QualificationError("slot_connection_closed")
        data.extend(chunk)
    return bytes(data)


def write_frame(connection: socket.socket, data: bytes, limit: int) -> None:
    if not 1 <= len(data) <= limit:
        raise QualificationError("slot_message_too_large")
    connection.sendall(len(data).to_bytes(4, "big") + data)


def assert_peer(connection: socket.socket) -> None:
    peer_option: object = getattr(socket, "SO_PEERCRED", None)
    if isinstance(peer_option, int):
        credentials = connection.getsockopt(socket.SOL_SOCKET, peer_option, 12)
        _, uid, _ = struct.unpack("3i", credentials)
        if uid != os.getuid():
            raise QualificationError("slot_peer_denied")
    elif sys.platform == "darwin":
        # Python does not expose getpeereid on every macOS build; libc does.
        uid_value, gid_value = ctypes.c_uint(), ctypes.c_uint()
        function = ctypes.CDLL(None).getpeereid
        function.argtypes = [
            ctypes.c_int,
            ctypes.POINTER(ctypes.c_uint),
            ctypes.POINTER(ctypes.c_uint),
        ]
        function.restype = ctypes.c_int
        if function(connection.fileno(), ctypes.byref(uid_value), ctypes.byref(gid_value)) != 0:
            raise QualificationError("slot_peer_identity_unavailable")
        if uid_value.value != os.getuid():
            raise QualificationError("slot_peer_denied")
    else:
        raise QualificationError("slot_peer_identity_unavailable")


class DeviceServer:
    def __init__(self, path: Path, dispatch: Callable[[SlotDeviceRequest], SlotDeviceResponse]):
        if path.parent != path.parent.resolve() or path.exists() or path.is_symlink():
            raise QualificationError("slot_socket_requires_recovery")
        self.path, self.dispatch = path, dispatch
        self.stopped = threading.Event()
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.bind(str(path))
        path.chmod(0o600)
        self.socket.listen(4)
        self.socket.settimeout(0.5)
        self.thread = threading.Thread(target=self._serve, daemon=True)

    def start(self) -> None:
        self.thread.start()

    def _serve(self) -> None:
        while not self.stopped.is_set():
            try:
                connection, _ = self.socket.accept()
            except TimeoutError:
                continue
            except OSError:
                return
            with connection:
                connection.settimeout(610)
                try:
                    assert_peer(connection)
                    request = SlotDeviceRequest.model_validate_json(
                        read_frame(connection, MAX_REQUEST), strict=True
                    )
                    response = self.dispatch(request)
                except Exception:
                    # Exceptions never serialize paths, customer data or tokens.
                    response = SlotDeviceResponse.model_validate(
                        {"ok": False, "reason": "slot_operation_failed"}
                    )
                try:
                    write_frame(connection, response.model_dump_json().encode(), MAX_RESPONSE)
                except (OSError, QualificationError):
                    pass

    def close(self) -> None:
        self.stopped.set()
        self.socket.close()
        self.thread.join(timeout=1)
        if self.thread.is_alive():
            raise QualificationError("slot_command_still_running")
        self.path.unlink(missing_ok=True)


def request(path: Path, payload: SlotDeviceRequest, timeout: int = 610) -> SlotDeviceResponse:
    details = path.stat(follow_symlinks=False)
    if (
        not stat.S_ISSOCK(details.st_mode)
        or details.st_uid != os.getuid()
        or stat.S_IMODE(details.st_mode) != 0o600
        or path.parent != path.parent.resolve()
    ):
        raise QualificationError("unsafe_slot_socket")
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
        connection.settimeout(timeout)
        connection.connect(str(path))
        assert_peer(connection)
        write_frame(connection, payload.model_dump_json().encode(), MAX_REQUEST)
        result = SlotDeviceResponse.model_validate_json(
            read_frame(connection, MAX_RESPONSE), strict=True
        )
    if not result.ok:
        raise QualificationError("slot_operation_failed")
    return result


def encoded(data: bytes) -> str:
    if len(data) > 16 * 1024 * 1024:
        raise QualificationError("slot_output_too_large")
    return base64.b64encode(data).decode("ascii")
