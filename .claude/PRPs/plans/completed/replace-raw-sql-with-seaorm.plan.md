# Plan: Replace raw backend SQL with SeaORM ORM access

Status: completed on 2026-09-24. The mandatory ledger is 33/33 converted; the final
source census is zero, the deny-style ORM boundary test passes, the historical migration
chain applies on a fresh database, all Rust workspace tests pass there, and the workspace
Clippy/build gates pass. See the implementation report for observed evidence.

## Summary

Convert `apps/api` runtime data access away from the `execution_store::{rows, one, exec}`
raw SQL helper layer and into typed SeaORM entities, query builders, active models and
relations. The first implementation phase must fill entity coverage for tables that
currently exist only in migrations, then migrate raw-heavy services in small behavioral
slices with route/integration tests preserved.

This is an internal backend refactor. It should not change browser, worker or public
contract shapes. The completion boundary includes every Rust file in `apps/api` that
currently contains or enables handwritten SQL: runtime data access becomes entity ORM,
tests become entity ORM, and migrations become SeaORM Migration/SeaQuery typed builders.

The supporting workspace audit and primary-source research are captured in
[`../../research/seaorm-orm-only-backend.research.md`](../../research/seaorm-orm-only-backend.research.md).

## User Story

As a backend maintainer, I want application data access expressed through SeaORM's typed
entities and query builders, so that column/table changes fail closer to compile time and
raw SQL strings stop bypassing Rust review.

## Problem -> Solution

Current runtime services use SeaORM connections but frequently issue handwritten SQL
through `Statement::from_sql_and_values`. Replace those runtime queries with typed
entity models, `Entity::find`, `QueryFilter`, `QueryOrder`, `QuerySelect`, `ActiveModel`
insert/update/delete, `OnConflict`, `lock_exclusive` and `lock_with_behavior`.

## Metadata

- **Complexity:** XL
- **Source PRD:** N/A
- **PRD Phase:** N/A
- **Raw-SQL census:** 35 workspace files: 33 in `apps/api` that must change, plus two
  non-SeaORM operational bootstrap/smoke files that are explicitly outside this Rust plan.
- **Estimated Files:** 50-65 source/test/migration files including new entity files
- **Implementation shape:** six coherent phases and 13 ordered tasks; land as reviewable
  increments while keeping the branch behaviorally complete at every merge boundary.
- **Research date:** 2026-09-24
- **Source baseline:** `dad922708860be753fb68a97886feccfaf110d07`
- **Graphify baseline:** `graphify-out/graph.json` built at `402e9caf826bf2f8ea99b5ad746e1adc7d2492f9`; graph results were verified against source.
- **External research:** Exa MCP was invoked but its free endpoint was rate-limited;
  primary SeaORM 2.0/docs.rs and SeaQL sources were used instead. No credential or
  user configuration was changed.
- **Confidence:** 7/10 for the application/test conversion and 6/10 overall because the
  same pass must preserve fresh-install and rollback behavior while converting 15 historical migrations.

## UX Design

N/A - internal backend refactor. The expected user-visible behavior is no change.

## Scope

Build:

- Add missing SeaORM entity files for raw-heavy runtime tables.
- Convert runtime code under `apps/api/src` away from `execution_store::{rows, one, exec}` and direct `query_*_raw` / `execute_raw`.
- Convert all four SQL-bearing API test files to entities or typed migration builders,
  including the legacy-migration harness; there is no raw-SQL test exception.
- Convert all 15 SQL-bearing historical migrations from `execute_unprepared` strings to
  SeaORM Migration/SeaQuery schema and query builders without changing their observable
  up/down behavior on a fresh database.
- Keep JSON payload decoding and public DTO mapping in services; database rows remain distinct from transport contracts.
- Add a deny-style guard test for new raw SQL anywhere under `apps/api` after conversion.
- Remove all API-test dependence on the production raw helper.

Not building:

- No public contract changes unless the compiler exposes a real mismatch.
- No global config changes.
- No `raw_sql!`, `find_by_statement`, `Statement`, `query_*_raw`, `execute_raw`,
  `Expr::cust` or SQL-string replacement hidden behind a new helper.
- No hosted/dev secrets, Doppler config or runtime process changes.
- No change to generated browser/worker consumer shapes.
- No attempt to route `infra/postgres/init.sql` or `scripts/runtime.py` through SeaORM:
  both run before/outside the Rust application connection boundary and are counted but
  cannot use Rust entities. They retain one narrowly scoped PostgreSQL statement each.

## Strategic Design

- **Approach:** complete the compact SeaORM entity graph first, then migrate behavior
  vertically by service cluster while preserving the existing domain-service/controller
  layout. Use entity queries and active models for database work, typed SeaQuery value/
  aggregate expressions only through entity builders, and bounded Rust composition for
  heterogeneous projections.
- **Runtime acceptance boundary:** all 14 inventoried runtime files are converted; zero
  raw SQL literals and zero raw SeaORM escape-hatch APIs remain in `apps/api/src`.
- **Test acceptance boundary:** all four inventoried test files use entities or typed
  migration builders; zero direct SQL remains in `apps/api/tests`.
- **Migration acceptance boundary:** all 15 inventoried migrations use SeaORM Migration/
  SeaQuery builders with typed `Iden` identifiers; zero `execute_unprepared`, `Statement`,
  or DDL/DML string literal remains in `apps/api/migration`.
- **Plan completion rule:** do not mark the refactor complete until every one of the 33
  checked backend files below satisfies its boundary and the repository guard passes.

Alternatives considered:

1. **Keep the helpers but hide SQL behind repositories** — rejected because table,
   column and result-shape strings would remain unchecked.
2. **Replace SQL strings with `raw_sql!`/`find_by_statement`** — rejected because bound
   parameters improve injection safety but do not meet the compile-time ORM goal.
3. **Use custom `Expr::cust` fragments for hard queries** — rejected for runtime code;
   use typed columns/relations or bounded Rust composition instead.
4. **Leave old migrations as raw SQL** — rejected by the expanded user requirement.
   Convert them with typed migration/query builders while preserving each migration's
   historical schema transition and validating both fresh-install and supported rollback paths.
5. **Switch every entity to SeaORM 2 dense format** — rejected as unrelated churn; the
   existing compact format is supported and already provides typed columns/relations.

## Mandatory Reading

| Priority | File                                                             |                   Lines | Why                                                                                      |
| -------- | ---------------------------------------------------------------- | ----------------------: | ---------------------------------------------------------------------------------------- |
| P0       | `docs/architect/README.md`                                       |                     all | Canonical documentation authority and status/source distinction.                         |
| P0       | `docs/architect/system.md`                                       |                    1-80 | Confirms `apps/api` owns Rust/Loco/SeaORM and persistence boundaries.                    |
| P0       | `docs/architect/system.md`                                       |                   70-79 | Explicitly says not to introduce a parallel hand-maintained SQL access layer.            |
| P0       | `docs/architect/development.md`                                  |                    1-18 | Finish scoped code/test/config before running validation.                                |
| P0       | `docs/architect/implementation/04-execution-and-reports.md`      |           31-58, 85-106 | Preserve idempotency, transactional claims, locks, fencing and reports.                  |
| P0       | `docs/architect/implementation/05-test-library-and-plans.md`     |                  47-117 | Preserve save/version, optimistic revision, mutation receipt and legacy-audit semantics. |
| P0       | `docs/architect/implementation/09-model-catalog-and-capacity.md` |                   26-85 | Preserve immutable definitions, assignment lifecycle and qualified-worker rules.         |
| P0       | `apps/api/src/services/execution_store.rs`                       |                    1-51 | Raw helper layer to retire from runtime use.                                             |
| P0       | `apps/api/src/models/_entities/mod.rs`                           |                    1-44 | Current entity inventory and missing modules.                                            |
| P1       | `apps/api/Cargo.toml`                                            |                    1-44 | Confirms package `mobile-qa` and `sea-orm = "2.0"`.                                      |
| P1       | `Cargo.lock`                                                     |               4363-4366 | Current locked SeaORM version is `2.0.2`.                                                |
| P1       | `apps/api/src/errors.rs`                                         |                   70-77 | Existing `DbErr` to `ApiFailure` mapping and logging style.                              |
| P1       | `apps/api/src/services/apps.rs`                                  |          18-44, 337-417 | Good local ORM filter, authorization and pagination patterns.                            |
| P1       | `apps/api/src/services/task_sessions.rs`                         | 61-80, 371-439, 520-625 | Good local ORM locking, update-many and `OnConflict` patterns.                           |
| P1       | `apps/api/src/services/capacity_wake.rs`                         |                   63-90 | Local `SKIP LOCKED` ORM pattern.                                                         |
| P1       | `apps/api/tests/large_artifacts.rs`                              |         78-158, 185-223 | ORM-based integration fixture pattern.                                                   |

## External Documentation

| Topic                                | Source                                                                                                                                    | Key Takeaway                                                                                                                                                                       |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Entity CRUD entry points             | https://docs.rs/sea-orm/2.0.2/sea_orm/entity/trait.EntityTrait.html                                                                       | `EntityTrait` provides typed `find`, `find_by_id`, `insert`, `insert_many`, `update`, `update_many`, `delete`, `delete_many` and `delete_by_id`.                                   |
| Filters, projections, joins, locking | https://docs.rs/sea-orm/2.0.2/sea_orm/query/trait.QuerySelect.html and https://docs.rs/sea-orm/2.0.2/sea_orm/query/trait.QueryFilter.html | Use query traits for select narrowing, joins, filters, group/having and row locks, including `lock_with_behavior`.                                                                 |
| Query builder overview               | https://docs.rs/sea-orm/2.0.2/sea_orm/query/index.html                                                                                    | Query builders compose through `QueryFilter`, `QuerySelect`, `QueryOrder` and `QueryTrait`.                                                                                        |
| SeaORM 2.0 direction                 | https://www.sea-ql.org/SeaORM/docs/introduction/whats-new/                                                                                | SeaORM 2.0 emphasizes the new entity format/entity-first workflow.                                                                                                                 |
| Typed projections and partial models | https://www.sea-ql.org/SeaORM/docs/basic-crud/select/                                                                                     | Use entity models, tuples or `DerivePartialModel` instead of string-indexed `QueryResult`; in 2.0 do not also derive `FromQueryResult` on a partial model.                         |
| Relations and joined models          | https://www.sea-ql.org/SeaORM/docs/relation/multi-selects/                                                                                | `find_also_related`, multi-selects and relations provide typed joined results; compact entities remain supported.                                                                  |
| Upsert/on conflict                   | https://docs.rs/sea-orm/2.0.2/sea_orm/query/struct.Insert.html and https://docs.rs/sea-orm/2.0.2/sea_orm/query/struct.TryInsert.html      | Use `OnConflict`/`TryInsert` instead of handwritten `ON CONFLICT`.                                                                                                                 |
| Transactions                         | https://www.sea-ql.org/SeaORM/docs/advanced-query/transaction/                                                                            | Preserve explicit short transactions and commit/rollback behavior.                                                                                                                 |
| Aggregates                           | https://www.sea-ql.org/SeaORM/docs/advanced-query/aggregate-function/                                                                     | Column methods provide typed count/sum/group/having projections.                                                                                                                   |
| Typed migrations                     | https://www.sea-ql.org/SeaORM/docs/migration/writing-migration/                                                                           | `SchemaManager` with SeaQuery `Table`, `Index`, `ForeignKey`, alter/drop builders and migration-local identifiers replaces handwritten DDL; migration seeding can use SeaORM APIs. |

## Current Raw SQL Inventory

### Exhaustive file census and completion ledger

The census was produced from the baseline commit with repository-wide searches for SQL
DML/DDL tokens, `execute_unprepared`, raw SeaORM statement APIs, and the central raw
helper's callers, followed by source inspection to remove false positives such as the
Rust method `.select(...)` and user-facing prose. Counts are files, not statements.

| Category                                     | Files now | Must change for this plan | Completion condition                                          |
| -------------------------------------------- | --------: | ------------------------: | ------------------------------------------------------------- |
| Runtime Rust (`apps/api/src`)                |        14 |                        14 | Entity/query-builder access only; raw helper surface removed. |
| API tests (`apps/api/tests`)                 |         4 |                         4 | Entity fixtures/assertions or typed migration builders only.  |
| Historical migrations (`apps/api/migration`) |        15 |                        15 | SeaORM Migration/SeaQuery builders only; behavior preserved.  |
| Operational PostgreSQL outside `apps/api`    |         2 |                         0 | Explicitly retained because they run before/outside SeaORM.   |
| **Total workspace census**                   |    **35** |                    **33** | **All 33 SeaORM-backend files checked below are converted.**  |

Runtime files — all 14 are mandatory:

- [ ] `apps/api/src/controllers/runs.rs`
- [ ] `apps/api/src/services/commercial.rs`
- [ ] `apps/api/src/services/execution_readiness.rs`
- [ ] `apps/api/src/services/execution_store.rs`
- [ ] `apps/api/src/services/model_registry.rs`
- [ ] `apps/api/src/services/run_baselines.rs`
- [ ] `apps/api/src/services/run_comparisons.rs`
- [ ] `apps/api/src/services/run_history.rs`
- [ ] `apps/api/src/services/test_authoring.rs`
- [ ] `apps/api/src/services/test_definitions.rs`
- [ ] `apps/api/src/services/test_library.rs`
- [ ] `apps/api/src/services/test_library_mutations.rs`
- [ ] `apps/api/src/services/test_library_save.rs`
- [ ] `apps/api/src/tasks/operator.rs`

API test files — all four are mandatory:

- [ ] `apps/api/tests/commercial.rs`
- [ ] `apps/api/tests/model_registry.rs`
- [ ] `apps/api/tests/task_sessions.rs`
- [ ] `apps/api/tests/test_library.rs`

Historical migration files — all 15 are mandatory:

- [ ] `apps/api/migration/src/m20260912_000001_app_setup.rs`
- [ ] `apps/api/migration/src/m20260912_000002_google_sign_in.rs`
- [ ] `apps/api/migration/src/m20260912_000003_workspaces.rs`
- [ ] `apps/api/migration/src/m20260912_000004_execution.rs`
- [ ] `apps/api/migration/src/m20260912_000005_test_library.rs`
- [ ] `apps/api/migration/src/m20260913_000006_task_sessions.rs`
- [ ] `apps/api/migration/src/m20260915_000007_execution_lifecycle.rs`
- [ ] `apps/api/migration/src/m20260920_000008_save_without_reviews.rs`
- [ ] `apps/api/migration/src/m20260920_000009_regression.rs`
- [ ] `apps/api/migration/src/m20260921_000010_model_registry.rs`
- [ ] `apps/api/migration/src/m20260921_000011_model_assignment.rs`
- [ ] `apps/api/migration/src/m20260921_000012_worker_poll_eligibility.rs`
- [ ] `apps/api/migration/src/m20260921_000014_commercial_access.rs`
- [ ] `apps/api/migration/src/m20260922_000015_credit_billing.rs`
- [ ] `apps/api/migration/src/m20260922_000016_credit_plan_changes.rs`

Counted but excluded operational files:

- `infra/postgres/init.sql` — creates the test database before the API/SeaORM connection exists.
- `scripts/runtime.py` — runs one PostgreSQL readiness/smoke probe from the process supervisor.

Those two exclusions are not hidden debt in the Rust ORM layer: neither file can import or
run SeaORM entities. Completion means 33/33 backend files converted, not an inaccurate
claim that a PostgreSQL database can be bootstrapped without a bootstrap statement.

### Runtime query clusters

The runtime raw SQL surface is concentrated here:

| Area                        | Evidence                                                                                                                                     | Conversion shape                                                                                                                                           |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Raw helper                  | `apps/api/src/services/execution_store.rs:22-51`                                                                                             | Keep pure helpers `json`, `decode`, `hash`, `word`, `conflict`; delete `field`, `rows`, `one`, `exec` after callers migrate.                               |
| Execution readiness         | `apps/api/src/services/execution_readiness.rs:38-44`                                                                                         | Filter `execution_profiles` by app and decode typed entity payloads.                                                                                       |
| Commercial access           | `apps/api/src/services/commercial.rs:43-66`, `90-118`, `187-210`, `377-423`, `540-661`, `674-693`                                            | Needs entities for `commercial_quotes`, `commercial_usage`, `commercial_audit`, `commercial_pilot_requests`; use active models, aggregates and locks.      |
| Model registry              | `apps/api/src/services/model_registry.rs:20-40`, `52-79`, `105-158`, `246-265`, `277-384`                                                    | Needs entities for `model_definitions`, `model_assignments`, `model_assignment_events`; decode active definitions and match typed contract fields in Rust. |
| Test definitions            | `apps/api/src/services/test_definitions.rs:34-63`, `83-148`, `170-235`                                                                       | Convert immutable definition/profile reads and idempotent insert to entities and `OnConflict`.                                                             |
| Test library reads          | `apps/api/src/services/test_library.rs:10-34`, `50-82`, `133-146`, `316-320`, `369-412`, `433-507`                                           | Needs `test_library_entries`, `test_library_drafts`, `test_library_defaults`; replace lateral/latest projection with staged typed queries if needed.       |
| Test library mutations/save | `apps/api/src/services/test_library_mutations.rs:49-60`, `93-125`, `204-245`, `273-324`; `apps/api/src/services/test_library_save.rs:65-102` | Use `ActiveModel`, `update_many`, `OnConflict`, and transaction-preserved idempotency.                                                                     |
| Authoring                   | `apps/api/src/services/test_authoring.rs:57-158`, `409-428`, `516-517`                                                                       | Replace locked session/task reads, counts, inserts and session payload updates with entities.                                                              |
| Run projections             | `apps/api/src/services/run_history.rs:34-38`, `run_baselines.rs:19-26`, `run_comparisons.rs:7-12`                                            | Use typed queries where possible; complex `UNION` may become two typed queries merged in Rust.                                                             |
| Controller/task checks      | `apps/api/src/controllers/runs.rs:87-94`, `apps/api/src/tasks/operator.rs:103-129`                                                           | Use entity joins or separate typed existence checks.                                                                                                       |

Migrations contain substantial raw DDL/backfill SQL, for example
`apps/api/migration/src/m20260912_000005_test_library.rs:9-76` and
`apps/api/migration/src/m20260921_000014_commercial_access.rs:10-94`. They are a separate
implementation phase because migration-time schema versions differ from current entities,
but all 15 remain blockers for this plan's expanded zero-raw-SQL backend completion gate.

## Missing Entity Coverage

These tables are referenced by runtime raw SQL or by ordinary raw test assertions but
currently have no entity module in `apps/api/src/models/_entities/mod.rs:1-44`:

| Table                        | Schema evidence                                                      | Needed for                                                      |
| ---------------------------- | -------------------------------------------------------------------- | --------------------------------------------------------------- |
| `execution_approvals`        | `apps/api/migration/src/m20260912_000004_execution.rs:15`            | Historical definition approval reads.                           |
| `execution_reviewer_grants`  | `apps/api/migration/src/m20260912_000004_execution.rs:14`            | Typed legacy-cleanup assertions; historical table completeness. |
| `test_library_entries`       | `apps/api/migration/src/m20260912_000005_test_library.rs:10-16`      | Catalog reads, mutation revisions, archive/default flows.       |
| `test_library_drafts`        | `apps/api/migration/src/m20260912_000005_test_library.rs:17-20`      | Draft save/load/upsert.                                         |
| `test_library_defaults`      | `apps/api/migration/src/m20260912_000005_test_library.rs:31-34`      | Default plan revision.                                          |
| `test_library_mutations`     | `apps/api/migration/src/m20260912_000005_test_library.rs:35-38`      | Idempotency receipts.                                           |
| `test_library_review_events` | `apps/api/migration/src/m20260912_000005_test_library.rs:25-30`      | Mostly migration/tests; add for completeness.                   |
| `model_definitions`          | `apps/api/migration/src/m20260921_000010_model_registry.rs:14-25`    | Model registry lifecycle.                                       |
| `model_assignments`          | `apps/api/migration/src/m20260921_000011_model_assignment.rs:13-26`  | Assignment staging/activation.                                  |
| `model_assignment_events`    | `apps/api/migration/src/m20260921_000011_model_assignment.rs:27-38`  | Assignment audit inserts.                                       |
| `commercial_quotes`          | `apps/api/migration/src/m20260921_000014_commercial_access.rs:35-52` | Quote validation and reservation.                               |
| `commercial_usage`           | `apps/api/migration/src/m20260921_000014_commercial_access.rs:53-70` | Commercial status, review, queued-cancel crediting.             |
| `commercial_audit`           | `apps/api/migration/src/m20260921_000014_commercial_access.rs:71-80` | Operator audit inserts.                                         |
| `commercial_pilot_requests`  | `apps/api/migration/src/m20260921_000014_commercial_access.rs:81-88` | Pilot request status and uniqueness.                            |

This is **14 new entity modules**: 12 required directly by runtime raw callers and two
legacy audit/grant modules required to remove raw ordinary-test assertions.
`test_library_versions` already exists and must be updated only with relations; retain
its post-migration nullable `legacy_review_state`.

## Unified Discovery Table

| Category        | File:Lines                                                | Pattern / finding                                                   | Implementation consequence                           |
| --------------- | --------------------------------------------------------- | ------------------------------------------------------------------- | ---------------------------------------------------- |
| Architecture    | `docs/architect/system.md:70-79`                          | SeaORM through Loco; no parallel hand-maintained SQL layer          | Runtime raw helper must be removed, not renamed      |
| Naming          | `apps/api/src/models/_entities/*.rs`                      | snake_case module/table, `Model`, `Entity`, `Column`, `ActiveModel` | New entities follow compact local format             |
| Error handling  | `apps/api/src/errors.rs:70-76`                            | `DbErr` logs a reason code and maps to safe 500                     | Use `?`; only translate known conflicts              |
| Logging         | `apps/api/src/errors.rs:70-76`; `services/uploads.rs:299` | structured `tracing` fields, no raw DB/user secrets                 | Refactor adds no SQL logging or payload dumps        |
| Transactions    | `services/auth.rs:183-223`                                | begin, lock, mutate, commit                                         | Preserve lock order and short transaction boundaries |
| Claim locks     | `services/capacity_wake.rs:66-92`                         | typed `SKIP LOCKED`                                                 | Use `lock_with_behavior`, never raw `FOR UPDATE`     |
| Compare-and-set | `services/uploads.rs:236-269`                             | typed filters + `rows_affected == 1`                                | Preserve optimistic/guarded updates                  |
| Relations       | `models/_entities/execution_attempts.rs:30-44`            | compact `DeriveRelation` + `Related`                                | Add only FK-backed relations required by callers     |
| Route tests     | `apps/api/tests/support/mod.rs:1-20,144-155`              | real PostgreSQL and HTTP boundary assertions                        | Keep behavior coverage, use entity fixtures          |
| Raw hub         | `services/execution_store.rs:17-54`                       | string fields and raw statements                                    | Delete `field`, `rows`, `one`, `exec` last           |

## Patterns to Mirror

### Entity filtering and authorization

Source: `apps/api/src/services/apps.rs:18-44`

```rust
memberships::Entity::find()
    .filter(memberships::Column::UserId.eq(user))
    .filter(memberships::Column::OrganizationId.eq(org))
    .filter(memberships::Column::Active.eq(true))
    .one(&ctx.db)
    .await?
    .ok_or_else(ApiFailure::missing)
```

Use `Entity::find`, `find_by_id`, chained `filter`, and `ok_or_else(ApiFailure::missing)`.
Preserve parent-scope checks in service code rather than relying on joins alone.

### Cursor pagination

Source: `apps/api/src/services/apps.rs:337-417`

Build a `Select`, add cursor filters with `Condition::any/all`, order by stable columns,
fetch `limit + 1`, truncate, and compute the next cursor from the last retained row.

### Transactions and locked reads

Source: `apps/api/src/services/task_sessions.rs:61-68`, `459-625`

```rust
let tx = ctx.db.begin().await?;
let row = Entity::find_by_id(id)
    .lock_exclusive()
    .one(&tx)
    .await?
    .ok_or_else(ApiFailure::missing)?;
// Apply typed state changes, then commit before external I/O.
tx.commit().await?;
```

Start with `ctx.db.begin().await?`, lock parent rows through `lock_exclusive()` or
`lock_with_behavior(LockType::Update, LockBehavior::SkipLocked)`, then commit after all
state changes and idempotency receipts are written.

### Bulk updates

Source: `apps/api/src/services/task_sessions.rs:70-80`; `apps/api/tests/large_artifacts.rs:145-158`

```rust
sessions::Entity::update_many()
    .col_expr(sessions::Column::RevokedAt, Expr::value(Utc::now()))
    .filter(sessions::Column::Id.eq(id))
    .exec(&ctx.db)
    .await?;
```

Use `Entity::update_many().col_expr(Column, Expr::value(...)).filter(...).exec(db)`.
Where the current code depends on row counts, inspect `UpdateResult.rows_affected`.

### Idempotent inserts

Source: `apps/api/src/services/task_sessions.rs:609-621`; `apps/api/src/services/capacity_control.rs:516-530`

```rust
Entity::insert(active_model)
    .on_conflict(
        OnConflict::column(Column::Id)
            .do_nothing()
            .to_owned(),
    )
    .try_insert()
    .exec(&tx)
    .await?;
```

Use `Entity::insert(active_model).on_conflict(OnConflict::column(...).do_nothing().to_owned()).exec_without_returning(db)`.
For update-on-conflict, use `OnConflict::columns(...).update_columns(...)` or explicit
`update_column` calls after verifying the exact SeaORM 2.0.2 API.

### Error handling

Source: `apps/api/src/errors.rs:70-77`; `apps/api/src/services/apps.rs:127-142`

```rust
impl From<sea_orm::DbErr> for ApiFailure {
    fn from(_error: sea_orm::DbErr) -> Self {
        tracing::error!(reason_code = "database_failure", "database operation failed");
        Self::internal()
    }
}
```

Let normal `DbErr` map to `ApiFailure::internal()` through `From<DbErr>`. Translate known
unique constraint conflicts into domain-specific 409 errors close to the insert/update.

### Test fixtures

Source: `apps/api/tests/large_artifacts.rs:78-158`

Build integration fixtures with entity `ActiveModel` inserts and `update_many`, not raw
helper calls. Keep route tests at the API boundary.

### Logging pattern

Source: `apps/api/src/services/uploads.rs:299`

```rust
tracing::info!(upload_id=%id, app_id=%app.id, phase="uploaded", "APK bytes sealed");
```

If the refactor needs new logs, use stable reason/phase fields and identifiers only;
never log SQL text, bound values, persisted payloads or secrets.

## Step-by-Step Tasks

Complete all code/test/config tasks before running any formatter, compiler, linter,
test, build or runtime check. The `VALIDATE` lines below identify final-batch coverage;
they are not instructions to run checks after each task.

### Phase 1 — Complete the entity graph

### Task 1: Add missing entity modules

- **ACTION:** Create entity files for every missing runtime table listed above and export them from `apps/api/src/models/_entities/mod.rs`.
- **IMPLEMENT:** Mirror the existing compact entity style: `//!` database-only comment, `DeriveEntityModel`, non-incrementing primary keys, `DateTimeUtc`, `Json`, `DeriveRelation`, `ActiveModelBehavior`.
- **MIRROR:** `apps/api/src/models/_entities/commercial_agreements.rs:1-31`, `apps/api/src/models/_entities/test_library_versions.rs:1-13`.
- **IMPORTS:** `sea_orm::entity::prelude::*`; no new crate dependency.
- **GOTCHA:** Composite primary keys are required for `execution_approvals`, `execution_reviewer_grants`, `model_definitions`, `model_assignments` and `test_library_mutations`. Match the **post-migration** column nullability/defaults exactly; do not infer from an earlier migration alone.
- **VALIDATE:** Final-batch `just check-api` compiles every entity and caller.

### Task 2: Add relations needed for joins and ownership checks

- **ACTION:** Add `belongs_to` / `has_many` relations only where runtime code needs typed relationship traversal or joins.
- **IMPLEMENT:** At minimum add relations among `execution_artifacts -> execution_attempts -> execution_runs`, `phone_tasks -> phone_sessions`, `commercial_usage -> commercial_quotes/commercial_agreements`, `test_library_versions -> test_library_entries/execution_definitions`, and relevant app ownership relations.
- **MIRROR:** `apps/api/src/models/_entities/execution_attempts.rs:29-41`, `apps/api/src/models/_entities/execution_runs.rs:20-29`.
- **IMPORTS:** Relation declarations use the existing entity prelude; service callers add `RelationTrait`/`QuerySelect` only when required.
- **GOTCHA:** Many current entities have `Relation {}` only. Do not invent relation semantics beyond actual FK constraints from migrations.
- **VALIDATE:** Final-batch affected integration tests execute every relation used by a service.

### Phase 2 — Convert historical migrations to typed builders

### Task 3: Convert all 15 raw-SQL migrations without schema drift

- **ACTION:** Replace every `execute_unprepared` SQL block in the 15 migration files in
  the completion ledger with SeaORM Migration `SchemaManager` operations and SeaQuery
  insert/update/select builders.
- **IMPLEMENT:** Define migration-local `DeriveIden` table/column enums; express tables,
  indexes, foreign keys, checks and column changes through `Table`, `Index`, `ForeignKey`
  and typed expressions. For data backfills whose source schema differs from current
  entities, use migration-local identifiers/models and SeaQuery builders rather than
  importing current runtime entities. Split multi-statement blocks into ordered builder
  calls inside the same migration transaction/connection.
- **MIRROR:** `apps/api/migration/src/m20260922_000017_credit_ledger.rs` and
  `m20260922_000018_credit_topups.rs`, which already use the migration schema DSL.
- **GOTCHA:** Preserve exact table/column names, PostgreSQL types, defaults, checks,
  partial/unique indexes, foreign-key actions, data backfills, validation failures and
  supported `down` ordering. Applied installations will not rerun edited migrations, but
  fresh databases and rollback/reapply tests must produce the same post-migration schema.
- **VALIDATE:** Final-batch migration tests, a fresh test-database migration to head, the
  existing save-without-reviews rollback/reapply scenario, and schema/introspection checks
  for constraints/indexes that route tests depend on.

### Phase 3 — Convert definitions, readiness and saved-library persistence

### Task 4: Convert execution readiness and `test_definitions`

- **ACTION:** Replace the raw profile scan in `execution_readiness.rs` and definition/profile reads plus import insert/select flow in `test_definitions.rs`.
- **IMPLEMENT:** Use `execution_profiles::Entity::find().filter(AppId.eq(app))`; use `execution_definitions::Entity::find_by_id(id).filter(AppId.eq(app))`; load `execution_approvals` ordered by purpose; insert definitions through `ActiveModel` with `OnConflict::columns([AppId, Kind, LogicalKey, Version]).do_nothing()`.
- **MIRROR:** ORM insert/update style from `apps/api/src/services/apps.rs:119-156`.
- **IMPORTS:** `execution_approvals`, `execution_definitions`, `execution_profiles`; `ActiveModelTrait`, `ColumnTrait`, `EntityTrait`, `QueryFilter`, `QueryOrder`, `Set`, `TransactionTrait`, and `sea_query::OnConflict` as used.
- **GOTCHA:** Preserve immutable-content conflict by re-reading existing row and comparing `content_hash` exactly as now.
- **VALIDATE:** Final-batch `execution` and `test_library` integration targets.

### Task 5: Convert test library catalog reads

- **ACTION:** Convert `apps/api/src/services/test_library.rs` raw catalog/draft/default/profile queries.
- **IMPLEMENT:** Use the new `test_library_*` entities. Prefer multiple typed reads plus Rust composition over lateral raw SQL for `entry()`.
- **MIRROR:** `apps/api/src/services/apps.rs:337-417` for list pagination and `apps/api/src/services/task_sessions.rs:83-94` for loading child records.
- **IMPORTS:** New library entities plus `Condition`, `EntityTrait`, `ColumnTrait`, `QueryFilter`, `QueryOrder`, `QuerySelect` and `PaginatorTrait` only where used.
- **GOTCHA:** Forward-schema payload handling must stay intact: invalid draft/current payload disables edit/default where current code does.
- **VALIDATE:** Final-batch `test_library` target covers catalog, detail, history, coverage and defaults.

### Task 6: Convert test library mutations and save

- **ACTION:** Replace mutation receipt, create/save/archive/default and link-import raw statements.
- **IMPLEMENT:** Use `test_library_entries::update_many` for revision bumps and archive toggles, entity inserts for entries/drafts/versions/mutations, and `OnConflict` for defaults/drafts/link-import.
- **MIRROR:** `apps/api/src/services/task_sessions.rs:426-439` for task insert plus parent update; `apps/api/src/services/capacity_control.rs:516-530` for idempotency insert.
- **IMPORTS:** New library entities; `ActiveModelTrait`, `IntoActiveModel`, `Set`, `EntityTrait`, `ColumnTrait`, `QueryFilter`, `TransactionTrait`, `sea_query::{Expr, OnConflict}`.
- **GOTCHA:** Preserve app-level transaction and receipt ordering from `test_library_mutations.rs:62-257`; a lost response must not create another version.
- **VALIDATE:** Final-batch `test_library` concurrency, retry, archive and save assertions.

### Phase 4 — Convert authoring and model routing

### Task 7: Convert authored task/session raw SQL

- **ACTION:** Replace raw session/task/environments/app reads in `apps/api/src/services/test_authoring.rs`.
- **IMPLEMENT:** Use `phone_sessions::Entity::find_by_id(id).lock_exclusive()`, `phone_tasks::Entity::find_by_id`, `phone_tasks::Entity::find().filter(...).count(...)`, and active-model inserts/updates.
- **MIRROR:** `apps/api/src/services/task_sessions.rs:61-80`, `371-439`.
- **IMPORTS:** `apps`, `environments`, `phone_sessions`, `phone_tasks`, new library entities; `ActiveModelTrait`, `EntityTrait`, `ColumnTrait`, `QueryFilter`, `PaginatorTrait`, `QuerySelect`, `Set`.
- **GOTCHA:** Keep the author/app join check for source tasks. Either add relations and join through `phone_sessions`, or do two typed reads with explicit equality checks.
- **VALIDATE:** Final-batch `task_sessions` and `test_library` targets.

### Task 8: Convert model registry

- **ACTION:** Replace all raw SQL in `apps/api/src/services/model_registry.rs`.
- **IMPLEMENT:** Use new `model_definitions`, `model_assignments`, `model_assignment_events` entities. Use locks for immutable revision and transition checks. Replace `MAX(revision)+1` with a typed descending revision lookup under the existing app lock. For provider/model matching, load active definitions, decode the bounded operator-owned contract values and filter in Rust; do not use a JSON-path string expression.
- **MIRROR:** `apps/api/src/services/capacity_wake.rs:67-89` for locked selection/update; `apps/api/src/services/task_sessions.rs:584-607` for multi-column update.
- **IMPORTS:** New model entities plus existing `apps`/`execution_workers`; `ActiveModelTrait`, `IntoActiveModel`, `EntityTrait`, `ColumnTrait`, `QueryFilter`, `QueryOrder`, `QuerySelect`, `Set`, `TransactionTrait`.
- **GOTCHA:** No `Expr::cust` exception is allowed. Preserve composite-key lookup, immutable payload hash, active/draining transition order, worker liveness and exact capability matching.
- **VALIDATE:** Final-batch `model_registry` and affected `task_sessions`/`execution` assertions.

### Phase 5 — Convert commercial and reporting projections

### Task 9: Convert commercial access

- **ACTION:** Replace raw SQL in `apps/api/src/services/commercial.rs` and `apps/api/src/tasks/operator.rs` commercial branches.
- **IMPLEMENT:** Add entities for quotes, usage, audit and pilot requests. Replace counts with `.count()` where simple, `select_only`/aggregate expressions where totals are needed, active-model inserts for quote/usage/audit, `update_many` for status/review changes, and locks for agreement/usage review.
- **MIRROR:** `apps/api/src/services/credit_billing.rs:557-560` for existing commercial agreement filters, plus `apps/api/src/services/apps.rs:18-44`.
- **IMPORTS:** New commercial entities plus `commercial_agreements`, `execution_attempts`, `execution_definitions`; `ActiveModelTrait`, `IntoActiveModel`, `EntityTrait`, `ColumnTrait`, `PaginatorTrait`, `QueryFilter`, `QueryOrder`, `QuerySelect`, `Set`, `TransactionTrait`.
- **GOTCHA:** The overlap check currently uses PostgreSQL `tstzrange` at `commercial.rs:547`. Prefer equivalent typed comparisons: an overlap exists when existing `starts_at < new_ends_at` and existing `ends_at > new_starts_at`.
- **VALIDATE:** Final-batch `commercial` target, especially overlap, totals, one-time review and crediting.

### Task 10: Convert run projections and artifact authorization

- **ACTION:** Replace raw SQL in `run_history`, `run_baselines`, `run_comparisons`, `controllers/runs.rs`, and operator run ownership checks.
- **IMPLEMENT:** Convert `run_comparisons` and artifact authorization to relation/existence queries. For each ordered baseline page, batch-load attempts for those run IDs, keep runs having at least one attempt and no unfinished attempt, then load build labels. Convert `run_history` into two typed queries (`execution_runs` and creator-private `phone_tasks` with sessions), apply the same cursor predicate to each, fetch at most 26 from each, merge by `(created_at DESC, id DESC)`, and truncate to 26 before constructing 25 items plus next cursor.
- **MIRROR:** `apps/api/src/services/apps.rs:382-397` for cursor filtering and stable ordering.
- **IMPORTS:** `builds`, `execution_artifacts`, `execution_attempts`, `execution_runs`, `phone_sessions`, `phone_tasks`; `Condition`, `EntityTrait`, `ColumnTrait`, `QueryFilter`, `QueryOrder`, `QuerySelect`, `RelationTrait`, `IntoActiveModel`, `Set` as needed.
- **GOTCHA:** Preserve history source filtering and cursor semantics exactly. The two-query replacement must still return at most 25 visible items and compute the same `created_at|id` cursor style.
- **VALIDATE:** Final-batch `execution` target plus focused history/baseline pagination assertions.

### Phase 6 — Convert fixtures, remove the escape hatch and enforce the boundary

### Task 11: Convert integration fixtures and add a raw-SQL guard

- **ACTION:** Replace raw helper usage in the four inventoried `apps/api/tests/*.rs` files
  and add `apps/api/tests/orm_boundary.rs` to fail on raw database APIs or SQL literals
  anywhere under `apps/api/src`, `apps/api/tests`, or `apps/api/migration/src`.
- **IMPLEMENT:** Use entity active models as in `apps/api/tests/large_artifacts.rs`. Rewrite
  the legacy-schema block at `test_library.rs:425-458` with migration schema builders and
  migration-local typed fixtures. The boundary test rejects `query_all_raw`, `query_one_raw`,
  `execute_raw`, `execute_unprepared`, `Statement::`, `raw_sql!`, `find_by_statement`,
  `Expr::cust`, and SQL DML/DDL/CTE string literals across the whole Rust backend.
- **MIRROR:** `apps/api/tests/large_artifacts.rs:78-158`.
- **IMPORTS:** Expand `apps/api/tests/support/mod.rs` entity re-exports only where shared; `orm_boundary.rs` uses `std::{fs, path::Path}` and no third-party dependency.
- **GOTCHA:** The legacy migration test still must construct the exact pre-migration schema
  and data; replacing its mechanism must not weaken its assertions or skip rollback behavior.
- **VALIDATE:** Final-batch `just check-api` includes the new boundary test and all converted fixtures.

### Task 12: Retire the raw `execution_store` surface

- **ACTION:** After all runtime and ordinary-test callers are migrated, delete `rows`, `one`, `exec` and `field` from `execution_store.rs`.
- **IMPLEMENT:** Keep `hash`, `json`, `decode`, `conflict`, and `word`; replace wildcard imports with explicit helper/entity imports so the compiler exposes leftovers.
- **MIRROR:** Existing service imports should move from `execution_store::*` to explicit entity imports.
- **IMPORTS:** Remove `ConnectionTrait`, `DatabaseBackend`, `QueryResult`, `Statement` and `Value` from `execution_store.rs`; update callers with their exact SeaORM traits.
- **GOTCHA:** Avoid a broad helper rename that hides raw access; the point is to remove raw query affordances from runtime code.
- **VALIDATE:** Final source scan and the boundary integration test return no runtime raw data access.

### Task 13: Update owning architecture documentation

- **ACTION:** Update `docs/architect/system.md` and `docs/architect/development.md`; after successful final validation, add only the observed evidence to `docs/architect/status.md`.
- **IMPLEMENT:** State that runtime app data access uses SeaORM entities/query builders and that the ORM-boundary test enforces it. Keep historical migration/backfill and local bootstrap SQL explicitly distinct. Record actual passing commands/date in status only after they run.
- **MIRROR:** `docs/architect/README.md:21-42` for document authority and planned/implemented wording.
- **IMPORTS:** N/A — documentation only.
- **GOTCHA:** Do not claim migration raw SQL is gone unless it actually is.
- **VALIDATE:** Final documentation link/consistency review; a post-check status evidence edit does not require rerunning unaffected application checks.

## Files to Change

| File/Area                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Action                  | Justification                                                                                            |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------- | -------------------------------------------------------------------------------------------------------- |
| `apps/api/src/models/_entities/execution_approvals.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | CREATE                  | Typed historical approval rows; composite primary key.                                                   |
| `apps/api/src/models/_entities/execution_reviewer_grants.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | CREATE                  | Typed historical reviewer-grant assertions; composite primary key.                                       |
| `apps/api/src/models/_entities/test_library_entries.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | CREATE                  | Typed catalog identity/revision/archive state.                                                           |
| `apps/api/src/models/_entities/test_library_drafts.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | CREATE                  | Typed mutable draft/upsert state.                                                                        |
| `apps/api/src/models/_entities/test_library_review_events.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | CREATE                  | Preserve typed legacy audit fixtures/history.                                                            |
| `apps/api/src/models/_entities/test_library_defaults.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | CREATE                  | Typed default-plan selection.                                                                            |
| `apps/api/src/models/_entities/test_library_mutations.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | CREATE                  | Typed composite-key idempotency receipts.                                                                |
| `apps/api/src/models/_entities/model_definitions.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | CREATE                  | Typed immutable model catalog revisions.                                                                 |
| `apps/api/src/models/_entities/model_assignments.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | CREATE                  | Typed composite-key assignment lifecycle.                                                                |
| `apps/api/src/models/_entities/model_assignment_events.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | CREATE                  | Typed assignment audit facts.                                                                            |
| `apps/api/src/models/_entities/commercial_quotes.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | CREATE                  | Typed quote identity and frozen pricing inputs.                                                          |
| `apps/api/src/models/_entities/commercial_usage.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | CREATE                  | Typed per-run reservation/review state.                                                                  |
| `apps/api/src/models/_entities/commercial_audit.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | CREATE                  | Typed commercial audit facts.                                                                            |
| `apps/api/src/models/_entities/commercial_pilot_requests.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | CREATE                  | Typed legacy pilot request record.                                                                       |
| `apps/api/src/models/_entities/mod.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | UPDATE                  | Export new entities.                                                                                     |
| `apps/api/src/models/_entities/{apps,users,builds,execution_definitions,execution_profiles,execution_runs,execution_attempts,execution_artifacts,phone_sessions,phone_tasks,test_library_versions,commercial_agreements}.rs`                                                                                                                                                                                                                                                                                                           | UPDATE AS NEEDED        | Add only FK-backed relations actually used by typed joins/loaders.                                       |
| `apps/api/src/services/execution_readiness.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | UPDATE                  | Replace raw profile scan.                                                                                |
| `apps/api/src/services/test_definitions.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | UPDATE                  | Replace raw definition/profile persistence.                                                              |
| `apps/api/src/services/test_library.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | UPDATE                  | Replace catalog/draft/default read SQL.                                                                  |
| `apps/api/src/services/test_library_mutations.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | UPDATE                  | Replace mutation and lifecycle write SQL.                                                                |
| `apps/api/src/services/test_library_save.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | UPDATE                  | Replace draft/version upsert SQL.                                                                        |
| `apps/api/src/services/test_authoring.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | UPDATE                  | Replace phone session/task raw SQL.                                                                      |
| `apps/api/src/services/model_registry.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | UPDATE                  | Replace registry and assignment SQL.                                                                     |
| `apps/api/src/services/commercial.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | UPDATE                  | Replace commercial quote/agreement/usage SQL.                                                            |
| `apps/api/src/services/run_history.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | UPDATE                  | Replace CTE/UNION raw query with typed query composition.                                                |
| `apps/api/src/services/run_baselines.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | UPDATE                  | Replace completed-run page query.                                                                        |
| `apps/api/src/services/run_comparisons.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | UPDATE                  | Replace finalization scan/update.                                                                        |
| `apps/api/src/controllers/runs.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | UPDATE                  | Replace artifact authorization raw join.                                                                 |
| `apps/api/src/tasks/operator.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | UPDATE                  | Replace operator commercial raw reads.                                                                   |
| `apps/api/migration/src/{m20260912_000001_app_setup,m20260912_000002_google_sign_in,m20260912_000003_workspaces,m20260912_000004_execution,m20260912_000005_test_library,m20260913_000006_task_sessions,m20260915_000007_execution_lifecycle,m20260920_000008_save_without_reviews,m20260920_000009_regression,m20260921_000010_model_registry,m20260921_000011_model_assignment,m20260921_000012_worker_poll_eligibility,m20260921_000014_commercial_access,m20260922_000015_credit_billing,m20260922_000016_credit_plan_changes}.rs` | UPDATE                  | Replace raw DDL/backfill SQL with typed SeaORM Migration/SeaQuery builders while preserving history.     |
| `apps/api/tests/{commercial,model_registry,task_sessions,test_library}.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                             | UPDATE                  | Convert every raw fixture/assertion; rewrite the legacy migration harness with typed migration builders. |
| `apps/api/tests/support/mod.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | UPDATE AS NEEDED        | Re-export entity/trait fixture building blocks.                                                          |
| `apps/api/tests/orm_boundary.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | CREATE                  | Enforce zero raw runtime database access.                                                                |
| `apps/api/src/services/execution_store.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | UPDATE                  | Delete raw helpers and string-indexed result extraction.                                                 |
| `docs/architect/system.md`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | UPDATE                  | Clarify ORM-only runtime/test boundary, typed migration boundary, and external bootstrap exclusion.      |
| `docs/architect/development.md`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | UPDATE                  | Document the enforced check and review boundary.                                                         |
| `docs/architect/status.md`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | UPDATE AFTER VALIDATION | Record only observed final validation evidence.                                                          |

## Testing Strategy

Targeted tests:

| Test target                                              | Why                                                                     |
| -------------------------------------------------------- | ----------------------------------------------------------------------- |
| `cargo test -p mobile-qa --test test_library --locked`   | Catalog, drafts, save/version lifecycle, migration compatibility tests. |
| `cargo test -p mobile-qa --test commercial --locked`     | Quote, agreement, usage, review and billing compatibility.              |
| `cargo test -p mobile-qa --test model_registry --locked` | Model definition/assignment lifecycle and worker capability checks.     |
| `cargo test -p mobile-qa --test task_sessions --locked`  | Phone session/task locking and idempotency.                             |
| `cargo test -p mobile-qa --test execution --locked`      | Run history/baseline/comparison and artifact/run flows.                 |
| `cargo test -p mobile-qa --test orm_boundary --locked`   | Prevent raw APIs/literals from returning anywhere in the Rust backend.  |
| `cargo test -p migration --locked`                       | Preserve migration up/down behavior and typed schema construction.      |
| `just check-api`                                         | Final API fmt/clippy/tests with isolated test DB, per project workflow. |

Edge cases:

- Immutable definition/model assignment revisions still reject changed payloads.
- Idempotency receipts replay same response and reject changed fingerprints.
- Archived library entries cannot be edited/defaulted.
- Commercial agreement periods do not overlap.
- Commercial usage cannot be reviewed twice.
- Queued-cancel credit still only credits runs never claimed by a device.
- `SKIP LOCKED` claim behavior remains non-blocking.
- Locked mutation/model/commercial transitions still serialize competing requests and
  return the same conflict/missing semantics.
- History merges runs/trials/legacy records in stable `(created_at, id)` order across
  page boundaries, including equal timestamps and source filters.
- Baseline selection skips runs with zero attempts or any unfinished attempt without
  letting an unrelated first page hide an older compatible run.
- Empty aggregates return zero, not missing/null, and amount sums keep their current
  integer width.
- Forward-schema JSON payload handling still disables unsafe edits without deleting data.
- Migration backfill tests construct legacy schema/data with typed migration/query builders.
- The ORM-boundary test rejects each forbidden API/token with a self-test fixture or
  table-driven matcher, while allowing ordinary comments and migration code.

## Validation Commands

Run once after completing the whole source/test/config/documentation batch. `just
check-api` already formats-checks, lints and runs the affected Rust test suite; use the
individual targets above only to rerun a failed/invalidated target after a batched fix.

```bash
just check-api
cargo build --workspace --locked
```

EXPECT: formatting, Clippy, all API/migration/unit/integration tests and the Rust
workspace build pass with the isolated local test database; no device/model work runs.

Raw-SQL guard command for review:

```bash
rg -n "query_.*_raw|execute_raw|Statement::from_sql_and_values|execute_unprepared|execution_store::(rows|one|exec)|\\b(SELECT|INSERT|UPDATE|DELETE|CREATE|ALTER|DROP|WITH)\\b" apps/api -g '*.rs'
```

Expected result: no handwritten SQL or raw SeaORM escape hatch in any Rust backend file.
False positives in comments/user-facing strings must be reviewed and, where practical,
the guard implementation should tokenize string literals instead of trusting grep alone.

Manual review after automated validation:

- Confirm migration rewrites preserve exact schema/backfill intent and that `git diff`
  contains no public contract/generated output, secret/config or unrelated formatting changes.
- Confirm every former `FOR UPDATE`, idempotent insert, optimistic revision update and
  affected-row check has an equivalent typed lock/conflict/filter operation.
- Confirm query fan-out is bounded: history fetches at most 26 rows per included source,
  baseline pages batch-load attempts/builds, and no per-row relation query was introduced.
- Add the actual commands/date/results to `docs/architect/status.md`, then perform a
  documentation consistency review only.

## Acceptance Criteria

- [ ] `apps/api/src` contains no `execution_store::{rows, one, exec,field}` usage,
      `Statement`, `raw_sql!`, `find_by_statement`, `query_*_raw`, `execute_raw`,
      `Expr::cust`, or SQL DML/CTE string literal.
- [ ] All 14 runtime files in the census ledger are converted and checked off.
- [ ] All four SQL-bearing API test files are converted and checked off; no raw-SQL
      compatibility-harness exception remains.
- [ ] All 15 historical migration files are converted and checked off; migration DDL and
      backfills use typed SeaORM Migration/SeaQuery builders and preserve fresh-install/up/down behavior.
- [ ] All 14 missing runtime/supporting tables have compact entity modules with exact post-migration
      primary keys, types, nullability and defaults; `mod.rs` exports them.
- [ ] `apps/api/tests` uses entities/active models or typed migration builders instead of
      the production raw helper or direct schema SQL.
- [ ] Query behavior, locks, idempotency and transaction boundaries match existing behavior.
- [ ] Complex history/baseline/library/model/commercial paths remain bounded and avoid
      N+1 database access.
- [ ] Route/integration tests covering affected API behavior pass.
- [ ] A guard check prevents new raw SQL across runtime, test and migration Rust code.
- [ ] Architecture docs accurately describe implemented ORM-only runtime persistence,
      typed migrations, and the two non-SeaORM operational SQL exclusions.

## Completion Checklist

- [ ] Every former raw call site in the inventory has a named typed replacement.
- [ ] Missing-row and unique-conflict mappings preserve the same public status/code.
- [ ] Database defaults remain `NotSet` on inserts unless the service intentionally
      supplies a single captured timestamp.
- [ ] Lock order and transaction boundaries match the original workflows; no external
      I/O occurs while a transaction is open.
- [ ] Affected-row checks remain for optimistic revisions and guarded state transitions.
- [ ] Public transport DTOs and generated browser/worker outputs are unchanged.
- [ ] The completion ledger is 33/33; no file is checked solely because its raw SQL was
      moved into another helper or module.
- [ ] Fresh-database migration plus supported rollback/reapply tests prove converted
      historical migrations retain their behavior.
- [ ] Final source scan, `just check-api`, and workspace build pass.
- [ ] Documentation records implemented behavior and observed evidence without claiming
      the two external operational PostgreSQL statements were converted.

## Risks

| Risk                                                        | Likelihood | Impact | Mitigation                                                                                                                                                      |
| ----------------------------------------------------------- | ---------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Missing or incorrect entity column type                     | Medium     | High   | Derive from migrations and compile against all affected tests before service migration.                                                                         |
| Behavior drift in complex projections                       | Medium     | Medium | Replace CTE/UNION/lateral SQL with small typed steps and preserve route tests.                                                                                  |
| JSON provider filtering loses an index or changes semantics | Medium     | Medium | Decode active operator-owned definitions and filter exact contract fields in Rust; measure before proposing normalized/indexed columns in a separate migration. |
| Commercial overlap/aggregate logic changes subtly           | Low-Medium | High   | Add focused tests around overlap, totals and review state.                                                                                                      |
| Historical migration behavior drifts during conversion      | Medium     | High   | Convert one migration at a time, mirror migration 017/018, and validate fresh install plus supported down/up paths before completion.                           |
| Typed composition introduces N+1 queries                    | Medium     | Medium | Batch by IDs, cap source pages, and review query count/shape for history and baselines.                                                                         |
| Lock/upsert translation changes race behavior               | Medium     | High   | Mirror proven local lock/`OnConflict` patterns and retain competing-request tests.                                                                              |

## Notes

- This plan treats historical migrations differently in implementation technique, not in
  completion status: runtime/tests use entities, while migrations use typed migration and
  query builders. All 33 Rust backend files remain mandatory.
- Do not use SeaORM `raw_sql!`, `find_by_statement`, `query_all_raw`, `query_one_raw` or
  `execute_raw` in runtime replacements; those APIs are documented and safe for parameters,
  but they do not satisfy this plan's type-checking goal.
- Prefer adding entity relations only when used. A giant relation sweep is easy to get wrong
  and harder to review.
- `infra/postgres/init.sql` and `scripts/runtime.py` are included in the 35-file workspace
  census for transparency. They are outside the SeaORM process boundary and therefore are
  not candidates for entity ORM conversion.
