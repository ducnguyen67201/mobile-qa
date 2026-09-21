# Mobile QA foundation

Read [docs/architect/README.md](docs/architect/README.md) before planning or implementation.
It is the canonical documentation authority. Update the owning document in the same PR
as a material change; use status.md to distinguish implemented behavior from planned work.

Finish ALL code/test/config for the agreed feature before formatting, typecheck,
lint, tests, builds, exports or runtime verification. Then validate once; batch fixes
and rerun only failed/invalidated checks. No check-on-save/lifecycle hooks, check
watchers or Rust rebuild watchers. Vite HMR only is permitted.

apps/api owns Loco/SeaORM, apps/web owns React/Vite, apps/mobile-worker owns Python/uv.
Root Cargo.toml is virtual. crates/contracts owns pure Rust transport source with no
Loco/DB dependency. Utoipa → local OpenAPI → Hey API SDK/types/Zod is the only browser
pipeline; Schemars → Pydantic is the worker pipeline. `just types` writes generated
outputs only when contents change. Never handwrite equivalent consumer shapes.
Keep actual API method/path/input/output/error agreement covered by real route tests.
Use generated Zod at success/error boundaries; do not trust SDK static types alone.
Preserve Loco scaffold markers. Spec 03 owns authentication/app/build setup; spec 02
owns explicit operator device qualification. Spec 04 owns HTTP worker leases and
reports; spec 05 owns editable drafts and reviewed test library versions.
No device/model calls in ordinary checks; no additional domain framework.
No global configuration changes. Keep secrets/artifacts out of Git. Do not touch
parent sources/. Coordinate shared files before parallel edits. Respect the existing
browser admin-policy denial; no alternate-access workaround for manual UI inspection.

Runtime secrets come from Doppler process injection. Do not create .env files or env
examples, download secrets to files, or commit tokens. Vite env-file loading is disabled.
Only wrap secret-consuming server processes; local checks/fake runs remain Doppler-free.
See docs/architect/environment.md. Do not change the user's Doppler auth/config without scope.

For local app sessions, run `just dev` from a configured checkout. It builds the API
once and starts PostgreSQL, the Doppler-injected API, Vite, the phone worker and the
execution worker. Do not wrap `just dev` itself in Doppler: its supervisor injects
secrets separately into the API and workers, while Vite gets no server secrets. Use
`just dev-logs` to inspect startup, `just dev-stop` to stop its owned processes, and
`just dev-restart` after source changes. If ports are occupied, identify their owners
and stop only services this task owns. A recovery journal can block a worker; inspect
its log and follow device recovery rather than deleting journals or reservations.
Restarting the API invalidates local sign-in sessions. See docs/architect/development.md.

Comments must make scaffold/fake/generated boundaries and non-obvious rules clear.
Follow docs/architect/commenting.md; explain purpose and constraints without narrating
every line. Keep comments aligned with implementation and preserve generator markers.

GAN design files are temporary working artifacts. After applying a GAN design run,
remove its Markdown specs, rubrics, state and feedback files from `gan-harness/`
before committing or updating a PR. Keep lasting decisions and acceptance limits
in the owning architecture document or implementation report instead.

For codebase relationship questions, consult the generated Graphify index when present:
`GRAPHIFY_QUERY_LOG_DISABLE=1 uv run --project tools/graphify --frozen graphify query "<question>"`.
Use `graphify explain "<symbol>"` or `graphify path "<A>" "<B>"` through the same uv prefix
for focused follow-up. Check `built_at_commit` in graphify-out/graph.json and inspect
source changes since that commit; feature branches and local edits can make it stale.
Verify graph findings in source. Fall back to normal search if the graph is missing,
stale or insufficient. Do not load the entire graph into context or rebuild it during
editing. docs/architect remains authoritative; this code-only index does not model
requirements or prove complete runtime relationships. CI owns the generated files.
