# Mobile QA foundation
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
Preserve Loco scaffold markers. No domain framework/device/model/auth in this phase.
No global configuration changes. Keep secrets/artifacts out of Git. Do not touch
parent sources/. Coordinate shared files before parallel edits. Respect the existing
browser admin-policy denial; no alternate-access workaround for manual UI inspection.
