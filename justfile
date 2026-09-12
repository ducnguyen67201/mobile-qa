set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list

setup:
    python3 scripts/setup.py

dev:
    python3 scripts/runtime.py dev

dev-web:
    pnpm --dir frontend dev

dev-api:
    cargo run --locked --bin mobile-qa-cli -- start --environment development

dev-worker-fake:
    uv run --no-sync --project workers/mobile --frozen mobile-qa-worker fake contracts/fixtures/pass.json

types:
    python3 scripts/contracts.py

check-contracts:
    python3 scripts/contracts.py --check
    python3 -m unittest scripts.test_contracts

check-web:
    pnpm --dir frontend typecheck
    pnpm --dir frontend lint
    pnpm --dir frontend test

check-api:
    python3 scripts/runtime.py check-api

check-worker:
    cd workers/mobile && uv run --no-sync --frozen ruff check .
    cd workers/mobile && uv run --no-sync --frozen pyright
    cd workers/mobile && uv run --no-sync --frozen pytest

check: check-contracts check-web check-api check-worker

build:
    pnpm --dir frontend build
    cargo build --workspace --locked

smoke:
    python3 scripts/runtime.py smoke

device-smoke:
    @echo "Spec 02 device qualification is not implemented. No device or cloud action was taken."
    @exit 2
