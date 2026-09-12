# Explicit development workflow

Finish the complete agreed implementation and tests before executing format,
typecheck, lint, test, build, generation or runtime verification. Then run a single
consolidated final phase. Batch related fixes and rerun failed/invalidated checks.
No editor check-on-save tasks, lifecycle validation hooks, check watchers, Rust
rebuild loops or global tool/editor configuration changes are included.

`just setup` fetches Cargo dependencies, pnpm dependencies (scripts disabled), and
uv dependencies including the optional SDK. Once lockfiles exist it uses frozen/
locked installs. It performs no application checks. Regular uv run commands use --no-sync; dependency
installation is confined to setup. The first authored checkout
needs `just types`; clean clones already contain generated outputs.

| Command | Work |
|---|---|
| types | Explicit staged contracts export, update only changed output |
| check-contracts | Regenerate to temp, compare drift; content-sync tests |
| check-web | Strict tsc, ESLint, Vitest once each |
| check-api | Cargo fmt check, Clippy, workspace tests against local test DB |
| check-worker | Ruff, strict Pyright on owned package, pytest |
| build | Vite production build; Cargo workspace debug build |
| smoke | Built API without dist, Vite proxy, DB bookkeeping, fixtures, import-only SDK |

Development and test use fixed isolated local DB URLs; they ignore inherited
DATABASE_URL. Both destructive flags remain false. The Loco request test does not
use automatic create/drop helpers. The checked database name is mobile_qa_test.
The Compose project is mobile-qa-local, with loopback-only binding, local-only
credentials and a persistent named volume. Initialization creates the separate
test database. DB startup never resets volumes. Stop an explicitly started DB with
`docker compose -f infra/compose.yaml stop postgres`.

Scripts stop only children they start, using owned process groups. An already running
Compose PostgreSQL is left running. API/Vite log files truncate on explicit start;
do not write payloads/secrets to logs. `.private/` is ignored and mode 0700. No real
customer credentials, APKs or artifacts belong in this setup baseline.

CI filters docs-only changes out of build jobs. API and infrastructure changes check
Rust/database; frontend changes check/build web; worker changes check Python. Shared
contracts and generation/tooling changes trigger all consumers and drift. Locks key
Cargo/pnpm/uv caches. No cloud/device/model tests run in CI. Hosted CI execution itself
must be observed after publication; local command success is not proof hosted CI ran.

Measured validation/start/HMR/export durations are recorded in the parent spec01
implementation report, with machine versions and explicit unavailable measurements.
Do not infer a latency promise from one machine. Cache reuse happens without cargo
clean or release builds in the ordinary loop.
