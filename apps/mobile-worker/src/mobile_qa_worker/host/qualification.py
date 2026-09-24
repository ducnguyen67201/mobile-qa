"""Offline report of recorded campaigns; never boot a device or manufacture qualification."""

import json
import math
from pathlib import Path
from typing import cast

from mobile_qa_worker.qualification.config import QualificationError, json_object
from mobile_qa_worker.qualification.evidence import atomic_json

MEASUREMENTS = (
    "boot_seconds",
    "install_seconds",
    "test_seconds",
    "rss_mib",
    "cpu_pressure",
    "disk_read_mib",
    "disk_write_mib",
    "cost_usd",
)
REQUIRED_SCENARIOS = {
    "large_apk",
    "graphics",
    "evidence",
    "cancellation",
    "phone_contention",
    "model_wait",
    "clean_reset",
}


def percentile(values: list[float], fraction: float) -> float:
    if not values or not 0 <= fraction <= 1:
        raise QualificationError("invalid_campaign_percentile")
    return sorted(values)[max(0, math.ceil(len(values) * fraction) - 1)]


def summarize(campaign: dict[str, object]) -> dict[str, object]:
    identities = (
        "instance_type",
        "region",
        "image_id",
        "toolchain_sha256",
        "profile_sha256",
        "apk_sha256",
        "rendering",
        "network_isolation_evidence",
    )
    if any(not isinstance(campaign.get(k), str) or not campaign[k] for k in identities):
        raise QualificationError("campaign_identity_missing")
    for key in ("toolchain_sha256", "profile_sha256", "apk_sha256"):
        value = cast(str, campaign[key])
        if len(value) != 64 or any(c not in "0123456789abcdef" for c in value):
            raise QualificationError("campaign_digest_invalid")
    count, samples = campaign.get("slots"), campaign.get("samples")
    if (
        type(count) is not int
        or not 1 <= count <= 8
        or not isinstance(samples, list)
        or not samples
    ):
        raise QualificationError("campaign_samples_missing")
    records = cast(list[object], samples)
    if len(records) > 10000:
        raise QualificationError("campaign_too_large")
    series: dict[str, list[float]] = {key: [] for key in MEASUREMENTS}
    scenarios: set[str] = set()
    failures = 0
    for raw in records:
        if not isinstance(raw, dict):
            raise QualificationError("campaign_sample_invalid")
        sample = cast(dict[str, object], raw)
        scenario = sample.get("scenario")
        if not isinstance(scenario, str) or scenario not in REQUIRED_SCENARIOS:
            raise QualificationError("campaign_scenario_invalid")
        scenarios.add(scenario)
        for key in MEASUREMENTS:
            value = sample.get(key)
            if (
                type(value) not in (int, float)
                or not math.isfinite(cast(float, value))
                or cast(float, value) < 0
            ):
                raise QualificationError("campaign_measurement_invalid")
            series[key].append(float(cast(float, value)))
        for flag in ("passed", "oom", "process_leak", "reset_contamination"):
            if type(sample.get(flag)) is not bool:
                raise QualificationError("campaign_result_invalid")
        if (
            sample["passed"] is not True
            or sample["oom"]
            or sample["process_leak"]
            or sample["reset_contamination"]
        ):
            failures += 1
    complete = scenarios == REQUIRED_SCENARIOS
    return {
        "version": 1,
        "kind": "recorded_capacity_campaign",
        "identity": {k: campaign[k] for k in identities},
        "candidate_slots": count,
        "sample_count": len(records),
        "failures": failures,
        "missing_scenarios": sorted(REQUIRED_SCENARIOS - scenarios),
        "eligible_for_operator_review": complete and failures == 0,
        "approved_slots": 0,
        "note": (
            "Measured for this app and image only. "
            "An operator must review evidence and approve capacity."
        ),
        "metrics": {
            k: {"p50": percentile(v, 0.50), "p95": percentile(v, 0.95)} for k, v in series.items()
        },
        "cost_per_completed_sample_usd": sum(series["cost_usd"]) / max(1, len(records) - failures),
    }


def report(campaign_path: Path, output: Path) -> None:
    if output != output.resolve() or output.exists():
        raise QualificationError("unsafe_capacity_report_path")
    output.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    result = summarize(json_object(campaign_path))
    # This report cannot be used directly as an approved profile or network attestation.
    atomic_json(output, result)
    print(json.dumps({"report": str(output), "approved_slots": 0}))
