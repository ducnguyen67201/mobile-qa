# SeaORM-only backend persistence implementation report

Date: 2026-09-24

## Outcome

The Rust backend persistence boundary is now ORM-only. All 33 mandatory files from the
plan's raw-SQL census were converted: 14 runtime/helper files, four API integration-test
files and 15 historical migrations. Repository scans find zero raw SeaORM statement APIs,
`execute_unprepared`, `Expr::cust`, or SQL-leading DDL/DML literals under `apps/api`.

The two counted operational statements remain intentionally outside this boundary:
`infra/postgres/init.sql` creates the database before the API connects, and
`scripts/runtime.py` owns a process-level PostgreSQL readiness probe.

## Implementation

- Added 14 SeaORM entity modules for execution approvals/reviewer grants, test-library
  entries/drafts/defaults/mutations/review events, model definitions/assignments/events,
  and commercial quotes/usage/audit/pilot requests.
- Replaced runtime projections, writes, locks, pagination and idempotency paths with
  entity queries, active models, typed filters/order/select traits and `OnConflict`.
- Removed `execution_store::{field, rows, one, exec}` and its raw `Statement` surface;
  the module retains only serialization, hashing, enum and domain-error helpers.
- Rewrote every SQL-bearing historical migration with `SchemaManager`, typed `Table`,
  `Index`, `ForeignKey` and query builders. Migration 000005 uses migration-local SeaORM
  entities for legacy validation/backfill and preserves the legacy PostgreSQL MD5 review
  event identifier exactly.
- Converted raw database setup/assertions in the four affected API tests to entities.
- Added `apps/api/tests/orm_boundary.rs` as a deny-style architectural regression test.

## Behavioral corrections found during validation

- Explicitly assigned changed JSON fields on converted active models; converting a model
  into an active model otherwise marks its values unchanged.
- Corrected the authoring environment lookup to filter by `app_id` instead of treating
  the app identifier as the environment table's primary key.

Both paths are covered by the passing test-library and task-session integration tests.

## Validation evidence

- `cargo fmt --all -- --check` — passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — passed.
- `cargo test --workspace --locked` — passed against a newly created disposable
  PostgreSQL database with `JAVA_HOME` set to the installed JDK 17 `Contents/Home`.
- The fresh run applied the complete migration chain and passed every Rust unit,
  integration, contract and doc test, including `orm_boundary`.
- `cargo build --workspace --locked` — passed.
- Raw-SQL API/literal census under `apps/api` — zero findings.
- `git diff --check` — passed.

The task-owned disposable database was removed after validation and the repository's
original test database configuration was restored. The retained shared test database was
not reset or altered.
