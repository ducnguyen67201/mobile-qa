"""Explicit app policies. Generic direct execution never uses the demo's backend or reset oracle."""

from mobile_qa_worker.device.android import AndroidDevice
from mobile_qa_worker.generated.models import ExecutionProfile
from mobile_qa_worker.qualification.config import Profile, QualificationError
from mobile_qa_worker.qualification.device import Device
from mobile_qa_worker.qualification.evidence import Evidence


def device_for(assignment: ExecutionProfile, host: Profile, evidence: Evidence) -> AndroidDevice:
    if assignment.image != host.system_image:
        raise QualificationError("host_profile_does_not_match_manifest")
    if (
        assignment.adapter == "demo_persistence_v1"
        and assignment.package == "ai.mobileqa.demo"
        and assignment.execution_context is None
    ):
        return Device(host, evidence)
    c = assignment.execution_context
    if (
        assignment.adapter != "android_direct_v1"
        or assignment.driver.value != "direct"
        or not assignment.qualified
        or c is None
        or c.schema_version != 1
        or c.adapter_revision != "android_direct_v1"
        or c.verifier_revision != "ui_v1"
        or c.worker_runtime_revision != "direct_v1"
        or c.package != assignment.package
        or c.qualified_profile_id != assignment.id
        or c.image != host.system_image
        or c.abi != host.abi
        or c.state_scope != "local_only"
        or c.width != 1080
        or c.height != 1920
        or c.density != 420
        or c.locale != "en-US"
        or c.timezone != "Etc/UTC"
        or not c.starting_checks
        or not c.launch_component.startswith(c.package + "/")
    ):
        raise QualificationError("unsupported_device_adapter")
    return AndroidDevice(host, evidence, assignment.package, c.launch_component)
