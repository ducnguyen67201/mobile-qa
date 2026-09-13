"""Fixed nine-run oracle plus interruption/recovery. Expected outcomes stay outside runner."""

import json
import subprocess
import sys
import tempfile
import time
import tomllib
from collections.abc import Callable, Iterator
from contextlib import contextmanager
from decimal import Decimal
from pathlib import Path
from typing import cast
from uuid import uuid4

from mobile_qa_worker.generated.models import QualificationResult
from mobile_qa_worker.qualification.config import (
    ACTIVITY,
    CASE,
    PACKAGE,
    SERIAL,
    QualificationError,
    parse_request,
)
from mobile_qa_worker.qualification.evidence import atomic_json, sha256
from mobile_qa_worker.qualification.process import stop_group
from mobile_qa_worker.qualification.runner import ROOT, fixture_ready, run_attempt


@contextmanager
def backend(mode: str) -> Iterator[None]:
    with tempfile.TemporaryDirectory(prefix="mobile-qa-backend-") as temporary:
        with owned_backend(mode, Path(temporary) / "ready"):
            yield


@contextmanager
def owned_backend(mode: str, ready: Path) -> Iterator[None]:
    child = subprocess.Popen(
        [
            sys.executable,
            str(ROOT / "infra/device-host/fixture_backend.py"),
            "--mode",
            mode,
            "--ready-file",
            str(ready),
        ],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )
    try:
        for _ in range(30):
            if child.poll() is not None:
                raise QualificationError("fixture_port_unavailable")
            if not ready.exists():
                time.sleep(0.1)
                continue
            try:
                fixture_ready()
                break
            except QualificationError:
                time.sleep(0.1)
        else:
            raise QualificationError("fixture_start_timeout")
        yield
    finally:
        stop_group(child)


def schedule() -> list[tuple[str, str, str]]:
    return [
        ("good", "ready", "passed"),
        ("broken", "ready", "failed"),
        ("good", "unavailable", "blocked"),
    ] * 3


def accepted(result: QualificationResult, expected: str) -> bool:
    data = result.model_dump(mode="json")
    return data["outcome"] == expected and data["reset"] == "verified_clean"


def cost_estimate(
    seconds: float,
    hourly: str | None,
    results: list[QualificationResult],
    input_rate: str | None,
    output_rate: str | None,
) -> dict[str, object]:
    def rate(value: str) -> Decimal:
        number = Decimal(value)
        if not number.is_finite() or number < 0:
            raise QualificationError("invalid_rate")
        return number

    host = rate(hourly) * Decimal(str(seconds)) / Decimal(3600) if hourly is not None else None
    model = Decimal(0)
    complete = True
    for result in results:
        if not result.usage and result.reason_code not in ("prerequisite_unavailable",):
            complete = False
        for usage in result.usage:
            if (
                input_rate is None
                or output_rate is None
                or usage.input_tokens is None
                or usage.output_tokens is None
            ):
                complete = False
                continue
            model += (
                rate(input_rate) * usage.input_tokens + rate(output_rate) * usage.output_tokens
            ) / 1000000
            complete &= usage.unknown_calls == 0
    return {
        "host_usd": str(host) if host is not None else None,
        "model_usd": str(model) if complete else None,
        "model_coverage": "complete" if complete else "unavailable_or_partial",
        "excludes": [
            "storage",
            "public_ipv4",
            "artifact_transfer",
            "host_time_outside_campaign",
            "human_time",
        ],
    }


def qualify(config_path: Path, run: Callable[..., QualificationResult] = run_attempt) -> int:
    config: dict[str, object] = tomllib.loads(config_path.read_text())
    required = {"good_apk", "broken_apk", "profile", "output_root", "max_seconds", "budget_usd"}
    optional = {
        "host_hourly_usd",
        "input_usd_per_million",
        "output_usd_per_million",
        "rate_source",
        "rate_date",
    }
    if not required <= config.keys() or config.keys() - required - optional:
        raise QualificationError("invalid_campaign")
    for key in required - {"max_seconds"} | optional:
        if key in config and not isinstance(config[key], str):
            raise QualificationError("invalid_campaign_type")
    if type(config["max_seconds"]) is not int or not 1 <= config["max_seconds"] <= 14400:
        raise QualificationError("invalid_campaign_budget")
    budget = Decimal(cast(str, config["budget_usd"]))
    if not budget.is_finite() or budget <= 0:
        raise QualificationError("invalid_campaign_budget")
    output = Path(cast(str, config["output_root"])).resolve() / ("campaign-" + str(uuid4()))
    output.mkdir(parents=True, mode=0o700)
    results: list[QualificationResult] = []
    expectations: list[str] = []
    started = time.monotonic()
    ok = True
    # Cancellation probe interrupts SDK process, then a fresh good attempt must work.
    scenarios = schedule() + [("good", "ready", "inconclusive"), ("good", "ready", "passed")]
    for index, (build, mode, expected) in enumerate(scenarios):
        elapsed = time.monotonic() - started
        estimate = cost_estimate(
            elapsed,
            cast(str | None, config.get("host_hourly_usd")),
            results,
            cast(str | None, config.get("input_usd_per_million")),
            cast(str | None, config.get("output_usd_per_million")),
        )
        known = sum(
            Decimal(str(estimate[k])) for k in ("host_usd", "model_usd") if estimate[k] is not None
        )
        if elapsed >= config["max_seconds"] or known >= budget:
            ok = False
            break
        apk = Path(cast(str, config[build + "_apk"])).resolve()
        request = parse_request(
            json.dumps(
                {
                    "version": 1,
                    "attempt_id": str(uuid4()),
                    "case_id": CASE,
                    "apk_path": str(apk),
                    "expected_apk_sha256": sha256(apk),
                    "package": PACKAGE,
                    "activity": ACTIVITY,
                    "serial": SERIAL,
                    "profile_path": str(Path(cast(str, config["profile"])).resolve()),
                    "output_root": str(output),
                }
            )
        )
        with backend(mode):
            result = run(request, interrupt_after=5 if index == 9 else None)
        results.append(result)
        expectations.append(expected)
        ok &= accepted(result, expected)
        if index == 9:
            ok &= result.reason_code == "cancelled"
        atomic_json(
            output / "campaign.json",
            {
                "status": "running",
                "attempts": [r.model_dump(mode="json") for r in results],
                "expected_outcomes": expectations,
            },
            replace=True,
        )
        if result.model_dump(mode="json")["reset"] != "verified_clean":
            break
    ok &= len(results) == 11
    estimate = cost_estimate(
        time.monotonic() - started,
        cast(str | None, config.get("host_hourly_usd")),
        results,
        cast(str | None, config.get("input_usd_per_million")),
        cast(str | None, config.get("output_usd_per_million")),
    )
    atomic_json(
        output / "campaign.json",
        {
            "status": "qualified" if ok else "rejected",
            "expected_outcomes": expectations,
            "attempts": [r.model_dump(mode="json") for r in results],
            "cost": estimate,
            "rate_source": config.get("rate_source"),
            "rate_date": config.get("rate_date"),
        },
        replace=True,
    )
    (output / "report.md").write_text(
        "# Device qualification\n\n"
        + ("Qualified" if ok else "Rejected")
        + f" — {len(results)}/11 attempts recorded.\n\n"
        "All attempts and cost coverage: campaign.json.\n"
        "This is a controlled demo experiment, not a production reliability claim.\n"
    )
    print(
        json.dumps(
            {"status": "qualified" if ok else "rejected", "report": str(output / "report.md")}
        )
    )
    return 0 if ok else 1
