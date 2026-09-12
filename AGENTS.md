# Mobile QA foundation
Finish ALL code for the agreed feature, including tests, before running any format,
typecheck, lint, test, build, export or runtime verification. Then validate once;
batch fixes and rerun only failed or invalidated checks. No check-on-save hooks,
type/lint/test watchers or Rust rebuild watchers. Vite HMR only is permitted.

Rust owns transport shapes in crates/contracts. Generated bindings/schema/Pydantic
are updated with `just types`, never handwritten. Preserve Loco scaffold markers.
Use SeaORM. No domain framework, cloud/device/model calls or auth in this phase.
No global configuration changes. Keep secrets and artifacts out of Git.
Do not touch parent sources/. Coordinate shared-file ownership before parallel edits.
