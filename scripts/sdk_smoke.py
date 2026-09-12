"""Check SDK import compatibility with Python socket connections blocked.

This narrow guard covers the import path only; it is not a sandbox for arbitrary
SDK execution. Never construct Agent here or treat import success as phone readiness.
"""
import json
import socket
from unittest.mock import patch

from mobile_qa_worker.cli import sdk_import


def blocked(*_args, **_kwargs):
    raise RuntimeError("SDK import attempted network access")


with patch.object(socket.socket, "connect", blocked), patch.object(socket.socket, "connect_ex", blocked), patch.object(socket, "create_connection", blocked):
    print(json.dumps(sdk_import(), sort_keys=True))
