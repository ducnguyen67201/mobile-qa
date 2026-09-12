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

check-web:
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
    uv sync --project apps/mobile-worker --frozen --extra sdk
    python3 scripts/local_device.py setup

device-local-build:
    python3 scripts/local_device.py build

# Real emulator + deterministic demo taps, no model credentials or API calls.
device-local scenario="good":
    python3 scripts/local_device.py run --scenario {{quote(scenario)}}

# Explicit Minitap run; model credentials come from the profile's Doppler config.
device-local-agent model scenario="good":
    python3 scripts/local_device.py run --scenario {{quote(scenario)}} --model {{quote(model)}}
