"""Execute one typed command on the owned serial. Mutating calls are never retried."""

import importlib
import re
import time
from collections.abc import Callable
from typing import Protocol, cast
from xml.etree import ElementTree

from mobile_qa_worker.device.android import AndroidDevice as Device
from mobile_qa_worker.generated.models import DirectCommand, DirectTarget, ExpectedCheck
from mobile_qa_worker.qualification.config import QualificationError


class ElementRpc(Protocol):
    def click(self, timeout: float = 0) -> object: ...
    def set_text(self, text: str, timeout: float = 0) -> object: ...


class DeviceRpc(Protocol):
    def __call__(self, **kwargs: str) -> ElementRpc: ...
    def swipe_ext(self, direction: str, scale: float = 0.7) -> object: ...
    def press(self, key: str) -> object: ...
    def dump_hierarchy(self, *, compressed: bool = False, max_depth: int = 50) -> str: ...


def device_rpc(device: Device) -> DeviceRpc:
    module = importlib.import_module("uiautomator2")
    if device.adb_server_port == 5037:
        return cast(DeviceRpc, module.connect(device.serial))
    adbutils = importlib.import_module("adbutils")
    client = adbutils.AdbClient(host="127.0.0.1", port=device.adb_server_port)
    return cast(DeviceRpc, module.connect(client.device(device.serial)))


def hierarchy(device: Device) -> bytes:
    if device.remote_commands:
        return device.hierarchy()
    # A running UiAutomator instrumentation owns accessibility. The shell dumper
    # cannot run alongside it; use the same service after the first direct RPC.
    if device.direct_automation:
        try:
            rpc = device_rpc(device)
            return rpc.dump_hierarchy(compressed=False, max_depth=50).encode()
        except Exception as exc:
            raise QualificationError("hierarchy_unavailable") from exc
    device.adb("shell", "uiautomator", "dump", "/data/local/tmp/mobile-qa.xml")
    return device.adb("exec-out", "cat", "/data/local/tmp/mobile-qa.xml")


def nodes(xml: bytes, package: str) -> list[ElementTree.Element]:
    if not xml or len(xml) > 1048576 or b"<!" in xml:
        raise QualificationError("invalid_hierarchy")
    try:
        root = ElementTree.fromstring(xml)
    except ElementTree.ParseError as exc:
        raise QualificationError("invalid_hierarchy") from exc
    if root.tag != "hierarchy":
        raise QualificationError("invalid_hierarchy")
    found = [n for n in root.iter("node") if n.get("package") == package]
    if not found:
        raise QualificationError("wrong_foreground_package")
    return found


def resolve(xml: bytes, package: str, target: DirectTarget, editable: bool = False) -> None:
    selector = target.root
    if not selector.value or len(selector.value) > 500:
        raise QualificationError("target_not_bound")
    key = "resource-id" if selector.by == "resource_id" else "content-desc"
    if key == "resource-id" and not selector.value.startswith(package + ":id/"):
        raise QualificationError("target_outside_app")
    matches = [n for n in nodes(xml, package) if n.get(key) == selector.value]
    if len(matches) != 1:
        raise QualificationError("target_not_unique")
    node = matches[0]
    if node.get("password") == "true" or node.get("enabled") != "true":
        raise QualificationError("target_not_available")
    if editable and not (node.get("editable") == "true" or "EditText" in node.get("class", "")):
        raise QualificationError("target_not_editable")
    bounds = re.fullmatch(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", node.get("bounds", ""))
    if not bounds:
        raise QualificationError("invalid_target_bounds")
    left, top, right, bottom = map(int, bounds.groups())
    if not (0 <= left < right <= 1080 and 0 <= top < bottom <= 1920):
        raise QualificationError("invalid_target_bounds")


def execute(
    device: Device,
    package: str,
    command: DirectCommand,
    rpc: DeviceRpc | None = None,
    stopped: Callable[[], bool] = lambda: False,
) -> None:
    if device.remote_commands:
        if stopped():
            raise QualificationError("task_stopped")
        device.execute_direct(command)
        return
    operation = command.root
    if stopped():
        raise QualificationError("task_stopped")
    wait_deadline = time.monotonic() + 10
    while True:
        tree = hierarchy(device)
        nodes(tree, package)
        try:
            if (
                operation.operation == "tap"
                or operation.operation == "set_text"
                or operation.operation == "wait_for"
            ):
                resolve(tree, package, operation.target, operation.operation == "set_text")
            break
        except QualificationError:
            if operation.operation != "wait_for" or time.monotonic() >= wait_deadline or stopped():
                raise
            time.sleep(0.25)
    if stopped():
        raise QualificationError("task_stopped")
    if operation.operation == "wait_for":
        return
    if operation.operation == "restart":
        try:
            device.adb("shell", "am", "force-stop", package)
            device.launch()
        except Exception as exc:
            raise QualificationError("device_action_uncertain") from exc
        return
    if rpc is None:
        rpc = device_rpc(device)
    device.direct_automation = True
    # A timeout after entering this block may have changed the app. Never retry it.
    try:
        if operation.operation == "tap" or operation.operation == "set_text":
            selector = operation.target.root
            element = rpc(
                **{
                    "resourceId" if selector.by == "resource_id" else "description": selector.value,
                    "packageName": package,
                }
            )
            if operation.operation == "tap":
                element.click(timeout=0)
            else:
                if len(operation.text.encode()) > 4000 or "\0" in operation.text:
                    raise QualificationError("invalid_text")
                element.set_text(operation.text, timeout=0)
        elif operation.operation == "back":
            rpc.press("back")
        elif operation.operation == "swipe":
            rpc.swipe_ext(operation.direction.value, scale=0.7)
    except QualificationError:
        raise
    except Exception as exc:
        raise QualificationError("device_action_uncertain") from exc


def check(
    device: Device,
    package: str,
    expected: ExpectedCheck,
    stopped: Callable[[], bool] = lambda: False,
) -> None:
    """Trial assertions use retained hierarchy properties, independently of AI prose."""
    deadline = time.monotonic() + min(60, max(1, expected.observation_seconds))
    while True:
        if stopped():
            raise QualificationError("task_stopped")
        tree = nodes(hierarchy(device), package)
        if not any(n.get("resource-id") == expected.ready_resource_id for n in tree):
            reason = "screen_not_ready"
        else:
            matches = [n for n in tree if n.get("resource-id") == expected.resource_id]
            if expected.text_filter:
                matches = [n for n in matches if n.get("text") == expected.text_filter]
            if expected.method.value == "ui_element_presence_v1":
                actual = "true" if matches else "false"
                reason = "assertion_failed" if actual != expected.expected else ""
            elif expected.method.value == "manual":
                raise QualificationError("manual_check_required")
            elif not matches:
                # A ready screen missing the expected element violates the assertion.
                # Multiple matches remain inconclusive because the target is ambiguous.
                reason = "assertion_failed"
            elif len(matches) > 1:
                reason = "target_not_unique"
            else:
                key = {
                    "text": "text",
                    "content_description": "content-desc",
                    "checked": "checked",
                    "enabled": "enabled",
                }[expected.property.value]
                reason = "" if matches[0].get(key) == expected.expected else "assertion_failed"
        if not reason:
            return
        if time.monotonic() >= deadline:
            raise QualificationError(reason)
        time.sleep(0.25)
