import boto3
from botocore.stub import Stubber
from mobile_qa_worker.generated.models import ObservedPower

from capacity_controller.aws import Power

INSTANCE = "i-0123456789abcdef0"


def test_adapter_targets_exact_instance_without_force_or_terminate(monkeypatch):
    client = boto3.client(
        "ec2", region_name="us-east-1", aws_access_key_id="test", aws_secret_access_key="test"
    )
    monkeypatch.setattr("capacity_controller.aws.boto3.client", lambda *a, **k: client)
    power = Power("us-east-1")
    with Stubber(client) as stub:
        stub.add_response(
            "describe_instances",
            {
                "Reservations": [
                    {"Instances": [{"InstanceId": INSTANCE, "State": {"Name": "stopped"}}]}
                ]
            },
            {"InstanceIds": [INSTANCE]},
        )
        stub.add_response("start_instances", {}, {"InstanceIds": [INSTANCE]})
        stub.add_response("stop_instances", {}, {"InstanceIds": [INSTANCE]})
        assert power.describe(INSTANCE) == ObservedPower.stopped
        power.start(INSTANCE)
        power.stop(INSTANCE)
        stub.assert_no_pending_responses()


def test_unknown_aws_state_never_becomes_stopped(monkeypatch):
    client = boto3.client(
        "ec2", region_name="us-east-1", aws_access_key_id="test", aws_secret_access_key="test"
    )
    monkeypatch.setattr("capacity_controller.aws.boto3.client", lambda *a, **k: client)
    power = Power("us-east-1")
    with Stubber(client) as stub:
        stub.add_response(
            "describe_instances",
            {
                "Reservations": [
                    {"Instances": [{"InstanceId": INSTANCE, "State": {"Name": "terminated"}}]}
                ]
            },
            {"InstanceIds": [INSTANCE]},
        )
        assert power.describe(INSTANCE) == ObservedPower.unknown
