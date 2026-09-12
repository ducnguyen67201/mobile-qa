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
