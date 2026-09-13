"""Exercise the actual CI filter data using picomatch (the paths-filter matcher)."""

import json
from pathlib import Path
import subprocess

import yaml

ROOT = Path(__file__).resolve().parents[1]
workflow = yaml.safe_load((ROOT / ".github/workflows/ci.yaml").read_text())
filters = yaml.safe_load(workflow["jobs"]["scope"]["steps"][1]["with"]["filters"])
cases = [
    (["crates/contracts/src/task_sessions.rs"], ["api", "web", "worker", "contracts"]),
    (["crates/contracts/src/task_sessions_api.rs"], ["api", "web", "contracts"]),
    (["scripts/task_session_smoke.py"], ["api", "worker"]),
    (["crates/contracts/src/test_library.rs"], ["api", "web", "contracts"]),
    (["crates/contracts/src/test_library_api.rs"], ["api", "web", "contracts"]),
    (["scripts/test_library_smoke.py"], ["api", "worker"]),
    (["crates/contracts/src/execution.rs"], ["api", "web", "worker", "contracts"]),
    (["scripts/execution_smoke.py"], ["api", "worker"]),
    (["infra/device-host/setup.sh"], ["worker"]),
    (["apps/mobile-worker/src/mobile_qa_worker/qualification/runner.py"], ["worker"]),
    (["apps/qa-demo-android/app/build.gradle"], ["demo"]),
    (["infra/postgres/init.sql"], ["api"]),
    (["crates/contracts/src/worker/qualification.rs"], ["api", "worker", "contracts"]),
    (["docs/architect/device-qualification.md"], []),
    (["apps/web/src/pages/Home.tsx"], ["web"]),
    (["infra/device-host/setup.sh", "apps/api/src/app.rs"], ["api", "worker"]),
    (["scripts/test_ci_scope.py"], ["scope_tests"]),
    (["scripts/local_device.py"], ["worker", "demo"]),
    (["scripts/setup_android.py"], ["api"]),
    (["scripts/apk_fixtures.py"], ["api"]),
    (["scripts/app_setup_smoke.py"], ["api"]),
    (
        [
            "apps/web/src/pages/Apps.tsx",
            "apps/mobile-worker/src/mobile_qa_worker/qualification/sdk_adapter.py",
        ],
        ["web", "worker"],
    ),
    ([".github/workflows/ci.yaml"], list(filters)),
]
# Resolve the exact installed matcher version from pnpm's locked dependency store.
matcher = next(
    (ROOT / "apps/web/node_modules/.pnpm").glob(
        "picomatch@4.0.7/node_modules/picomatch"
    )
)
script = """
const fs = require('fs');
const {filters, cases, matcher} = JSON.parse(fs.readFileSync(0, 'utf8'));
const match = require(matcher);
for (const [paths, expected] of cases) {
  const actual = Object.keys(filters).filter(job => paths.some(path => filters[job].some(pattern => match(pattern, {dot:true})(path))));
  if (JSON.stringify(actual.sort()) !== JSON.stringify(expected.sort())) throw new Error(JSON.stringify({paths, actual, expected}));
}
"""
subprocess.run(
    ["node", "-e", script],
    input=json.dumps({"filters": filters, "cases": cases, "matcher": str(matcher)}),
    text=True,
    check=True,
)
print(
    f"{len(cases)} CI scope cases passed (including deletion paths and combined PR changes)."
)
