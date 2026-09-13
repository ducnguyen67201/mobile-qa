# Explicit commands only: finish the scoped edits before types/check/build/smoke.
# setup installs dependencies; dev starts services; neither attaches check watchers.
# Command behavior and environment selection: docs/architect/development.md.
set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list

setup:
    python3 scripts/setup.py

dev:
    python3 scripts/runtime.py dev

dev-web:
    pnpm --dir apps/web dev

dev-api:
    cd apps/api && doppler run --no-fallback --forward-signals -- cargo run --locked --bin mobile-qa-cli -- start --environment development

dev-worker-fake:
    uv run --no-sync --project apps/mobile-worker --frozen mobile-qa-worker fake contracts/fixtures/pass.json

types:
    python3 scripts/contracts.py

check-contracts:
    python3 scripts/contracts.py --check
    python3 -m unittest scripts.test_contracts

format:
    python3 scripts/format.py

format-check:
    python3 scripts/format.py --check

check-web:
    pnpm --dir apps/web format:check
    pnpm --dir apps/web typecheck
    pnpm --dir apps/web lint
    pnpm --dir apps/web test

check-api:
    python3 scripts/runtime.py check-api

check-worker:
    cd apps/mobile-worker && uv run --no-sync --frozen ruff check .
    cd apps/mobile-worker && uv run --no-sync --frozen pyright
    cd apps/mobile-worker && uv run --no-sync --frozen pytest

check: check-contracts check-web check-api check-worker

build:
    pnpm --dir apps/web build
    cargo build --workspace --locked

smoke:
    python3 scripts/runtime.py smoke

# Explicit device work only; never dependencies of setup/check/build/smoke.
device-doctor profile:
    uv run --no-sync --project apps/mobile-worker --frozen mobile-qa-worker device-doctor --profile {{quote(profile)}}

device-smoke request:
    uv run --no-sync --project apps/mobile-worker --frozen mobile-qa-worker device-run --request {{quote(request)}}

device-qualify config:
    uv run --no-sync --project apps/mobile-worker --frozen mobile-qa-worker device-qualify --config {{quote(config)}}

# Explicit real-device development. Ordinary dev/check/smoke never boots a phone.
device-local-setup:
    uv sync --project apps/mobile-worker --frozen --extra sdk --extra device --extra ai
    python3 scripts/local_device.py setup

device-local-build:
    python3 scripts/local_device.py build

# Real emulator + deterministic demo taps, no model credentials or API calls.
device-local scenario="good":
    python3 scripts/local_device.py run --scenario {{quote(scenario)}}

# Explicit Minitap run; model credentials come from the profile's Doppler config.
device-local-agent model scenario="good":
    python3 scripts/local_device.py run --scenario {{quote(scenario)}} --model {{quote(model)}}

setup-android:
    python3 scripts/setup_android.py

apk-fixtures:
    python3 scripts/apk_fixtures.py

smoke-app-setup:
    python3 scripts/app_setup_smoke.py

# Explicit HTTP worker commands. Simulated profiles never import the device SDK.
dev-execution-fake origin profile_id state:
    uv run --no-sync --project apps/mobile-worker --frozen mobile-qa-worker execution-worker --origin {{quote(origin)}} --profile-id {{quote(profile_id)}} --state {{quote(state)}}

smoke-execution:
    python3 scripts/execution_smoke.py

# Real emulator/Minitap execution is explicit; inject MOBILE_QA_WORKER_TOKEN first.
dev-execution-real origin profile_id state profile:
    uv run --no-sync --project apps/mobile-worker --frozen mobile-qa-worker execution-worker --origin {{quote(origin)}} --profile-id {{quote(profile_id)}} --state {{quote(state)}} --profile {{quote(profile)}}

# Full author/review/default/run HTTP flow; fake Python evidence, no phone or secrets.
smoke-test-library:
    python3 scripts/test_library_smoke.py

# Typed direct definitions through the real API; simulated evidence, no model calls.
smoke-direct-authoring:
    python3 scripts/direct_authoring_smoke.py
