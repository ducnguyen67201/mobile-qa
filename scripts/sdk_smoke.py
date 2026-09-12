"""Import-only gate: block outbound network before loading the optional SDK."""
import json
import socket
from unittest.mock import patch

from mobile_qa_worker.cli import sdk_import


def blocked(*_args, **_kwargs):
    raise RuntimeError("SDK import attempted network access")


with patch.object(socket.socket, "connect", blocked), patch.object(socket.socket, "connect_ex", blocked), patch.object(socket, "create_connection", blocked):
    print(json.dumps(sdk_import(), sort_keys=True))
