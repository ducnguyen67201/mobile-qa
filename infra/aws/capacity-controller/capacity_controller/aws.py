"""The only runtime AWS power adapter. There is deliberately no terminate method."""

from typing import TYPE_CHECKING, cast

import boto3
from botocore.config import Config
from mobile_qa_worker.generated.models import ObservedPower

if TYPE_CHECKING:
    from mypy_boto3_ec2 import EC2Client


class Power:
    def __init__(self, region: str) -> None:
        # An automatic SDK retry of a timed-out write would escape the operation
        # journal. The next reconciliation observes outcome instead of repeating it.
        self.client = cast(
            "EC2Client",
            boto3.client(
                "ec2",
                region_name=region,
                config=Config(
                    connect_timeout=2,
                    read_timeout=3,
                    retries={"total_max_attempts": 1},
                ),
            ),
        )

    def describe(self, instance_id: str) -> ObservedPower:
        response = self.client.describe_instances(InstanceIds=[instance_id])
        instances = [i for r in response.get("Reservations", []) for i in r.get("Instances", [])]
        if len(instances) != 1 or instances[0].get("InstanceId") != instance_id:
            return ObservedPower.unknown
        name = instances[0].get("State", {}).get("Name", "unknown")
        try:
            return ObservedPower(name)
        except ValueError:
            return ObservedPower.unknown

    def start(self, instance_id: str) -> None:
        self.client.start_instances(InstanceIds=[instance_id])

    def stop(self, instance_id: str) -> None:
        # Graceful ACPI shutdown, with force/skip-OS-shutdown deliberately absent.
        self.client.stop_instances(InstanceIds=[instance_id])
