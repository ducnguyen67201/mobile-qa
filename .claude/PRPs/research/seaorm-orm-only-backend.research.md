# SeaORM ORM-only backend: research report

_Generated: 2026-09-24 | Scope: `apps/api` persistence | Confidence: high for the
workspace inventory, medium-high for effort estimates_

## Executive summary

The backend already has a solid SeaORM pattern, but it is split in two. Newer services
use entities, active models, typed filters, transactions, row locks and conflict
builders. Thirteen runtime callers instead call three generic helpers in
`apps/api/src/services/execution_store.rs`, which accept arbitrary SQL strings and
return name-indexed `QueryResult` values. That helper layer defeats compile-time checks
for table names, columns, result shapes and value conversions. Including that helper,
the production runtime raw-SQL surface is exactly 14 files.

The migration is larger than replacing three helper functions. Twelve tables used by
runtime callers have no SeaORM entity, two additional legacy audit/grant tables need
entities to remove ordinary-test raw assertions, several multi-table reads depend on missing
relations, and the run-history query combines two record families with a PostgreSQL CTE
and `UNION ALL`. The safest design is an entity-first, behavior-preserving migration in
coherent phases: complete the entity graph, move simple CRUD and locks, move the test
library/model/commercial transaction clusters, replace complex SQL projections with
typed ORM queries plus bounded Rust composition, convert ordinary test fixtures, then
delete the raw helpers and enforce the boundary.

The complete census is 35 files: 14 runtime files, four API test files, 15 historical
migrations, and two operational PostgreSQL files outside the Rust application. The
expanded requirement makes all 33 files under `apps/api` mandatory conversion targets.
Runtime/tests should use entity ORM (or migration-local typed fixtures); historical
migrations should use SeaORM Migration/SeaQuery typed builders. The two external files
remain explicit exceptions because they create/probe PostgreSQL before or outside a
SeaORM application connection.

## Research questions

1. Where does raw SQL enter the production backend, and which behaviors does it own?
2. Which tables and relations are missing from the SeaORM entity graph?
3. Which existing repository patterns prove the required ORM features already work?
4. Can SeaORM 2.0.2 express the needed reads, writes, aggregates, upserts and locks?
5. Which raw SQL is not runtime application data access and should remain out of scope?

## Workspace findings

### Raw runtime access hub

`apps/api/src/services/execution_store.rs:22-54` defines the complete generic raw
access layer:

| Helper | SeaORM escape hatch                                  | Type-safety loss                              |
| ------ | ---------------------------------------------------- | --------------------------------------------- |
| `rows` | `query_all_raw(Statement::from_sql_and_values(...))` | Arbitrary SQL and name-indexed rows           |
| `one`  | `query_one_raw(...)`                                 | Missing-row semantics plus string field names |
| `exec` | `execute_raw(...)`                                   | Arbitrary write SQL; only row count is typed  |

`field` at lines 17-19 compounds the issue by converting `QueryResult` columns through
runtime string names. The file's hashing, JSON encode/decode, conflict and enum-string
helpers are not raw database access and can remain after the query helpers are removed.

### Runtime clusters

| Area                      | Primary file                         | Raw call sites found | Main conversion                                                    |
| ------------------------- | ------------------------------------ | -------------------: | ------------------------------------------------------------------ |
| Readiness                 | `services/execution_readiness.rs`    |                    1 | Entity filter + JSON decode                                        |
| Test definitions/profiles | `services/test_definitions.rs`       |                    8 | Entities, active models, app lock, approval relation               |
| Test-library reads        | `services/test_library.rs`           |                   14 | Missing entities, typed joins, bounded Rust hydration              |
| Test-library mutations    | `services/test_library_mutations.rs` |                   16 | Transactional active models, optimistic update, upserts            |
| Test-library save         | `services/test_library_save.rs`      |                    5 | Immutable insert, mapping insert, draft upsert                     |
| Phone authoring           | `services/test_authoring.rs`         |                   13 | Existing phone entities plus library entities                      |
| Model registry            | `services/model_registry.rs`         |                   21 | Three missing entities, JSON decoding, row locks                   |
| Commercial legacy access  | `services/commercial.rs`             |                   28 | Four missing entities, aggregates, overlap predicate, audit writes |
| Run history               | `services/run_history.rs`            |      1 complex query | Two typed queries, merge/paginate in Rust                          |
| Baseline choices          | `services/run_baselines.rs`          |      1 complex query | Typed run/attempt/build filtering                                  |
| Comparison finalization   | `services/run_comparisons.rs`        |                    2 | Entity select + guarded update                                     |
| Artifact authorization    | `controllers/runs.rs`                |                    1 | Artifact/attempt relation lookup                                   |
| Operator task             | `tasks/operator.rs`                  |                    2 | Typed run and pilot-request queries                                |

The raw SQL surface is confined to 13 runtime callers plus the helper: 14 files total.
Other services may
import `execution_store`, but only for serialization, hashing, conflict construction or
enum-string conversion.

### Missing entities

The schema owns these runtime/supporting tables, but `apps/api/src/models/_entities` does not:

- `execution_approvals`
- `execution_reviewer_grants` (test/historical completeness)
- `test_library_entries`
- `test_library_drafts`
- `test_library_review_events`
- `test_library_defaults`
- `test_library_mutations`
- `model_definitions`
- `model_assignments`
- `model_assignment_events`
- `commercial_quotes`
- `commercial_usage`
- `commercial_audit`
- `commercial_pilot_requests`

`test_library_review_events` is likewise test/historical completeness rather than a
current runtime query dependency. The implementation plan therefore creates 14 entity
modules in total: 12 runtime requirements plus these two historical/test surfaces.

Their authoritative column definitions are in migrations 000004, 000005, 000008,
000010, 000011 and 000014. Existing `test_library_versions` already reflects migration
000008's nullable `legacy_review_state` and must be related to the new entry/definition
entities rather than regenerated from the original 000005 shape.

### Existing ORM patterns to reuse

- `services/apps.rs:18-44` uses `Entity::find`, typed filters and `.one()` for
  tenant-scoped authorization.
- `services/apps.rs:116-145` inserts active models within an explicit transaction and
  maps unique-constraint errors to stable API failures.
- `services/auth.rs:183-223` combines `OnConflict`, `try_insert`, `lock_exclusive`,
  active-model update and commit for a concurrency-safe workflow.
- `services/capacity_wake.rs:66-92` uses `lock_with_behavior(Update, SkipLocked)` and
  active-model updates for a claim loop.
- `services/uploads.rs:236-269` uses `update_many`, typed `col_expr` values, filters and
  `rows_affected` for compare-and-set semantics.
- `models/_entities/execution_attempts.rs:30-44` shows the compact entity relation style
  already used by this repository.
- `errors.rs:70-76` centrally converts `DbErr` into a safe API response and structured
  database-failure log.

### Exhaustive file-count boundary

- Four API integration-test files use the runtime `rows`/`one`/`exec` helpers for fixture
  setup and assertions: `commercial.rs`, `model_registry.rs`, `task_sessions.rs`, and
  `test_library.rs`. All four must move to entities or typed migration builders.
- Fifteen historical migration files contain 41 `execute_unprepared` calls. Migrations
  000017 and 000018 demonstrate the preferred SeaORM Migration schema-builder style.
  The 15 raw migrations all remain completion blockers under the expanded scope.
- `apps/api/tests/test_library.rs:425-458` intentionally builds a legacy schema and
  exercises old migrations. Preserve the scenario, but express its temporary schema and
  fixture operations through typed migration/query builders rather than raw strings.
- `infra/postgres/init.sql` and `scripts/runtime.py:182-199` bootstrap/check the local
  PostgreSQL environment. They are the two counted exclusions because they cannot load
  Rust SeaORM entities at the point where they execute.
- `crates/contracts` and `apps/mobile-worker` do not access the database.

## Official SeaORM findings

The repository resolves SeaORM **2.0.2** in `Cargo.lock`; implementation should use
that pinned API rather than assume later patch behavior.

1. `EntityTrait` supplies typed find/insert/update/delete entry points, including
   composite primary keys, insert-many, update-many and delete-many. This covers the
   simple raw helpers without a second data layer. [SeaORM 2.0.2 API](https://docs.rs/sea-orm/2.0.2/sea_orm/entity/trait.EntityTrait.html)
2. `QueryFilter`, `QuerySelect`, `QueryOrder` and `PaginatorTrait` compose typed
   conditions, projections, joins, grouping, limits and pagination on entity queries.
   [Query module](https://docs.rs/sea-orm/2.0.2/sea_orm/query/index.html),
   [QueryFilter](https://docs.rs/sea-orm/2.0.2/sea_orm/query/trait.QueryFilter.html),
   [QuerySelect](https://docs.rs/sea-orm/2.0.2/sea_orm/query/trait.QuerySelect.html)
3. Partial models and tuple/model selectors provide typed result shapes for projections;
   they should replace string-indexed `QueryResult` extraction. SeaORM 2.0 partial
   models implement result decoding directly and should not also derive
   `FromQueryResult`. [Select and partial models](https://www.sea-ql.org/SeaORM/docs/basic-crud/select/)
4. Relations and multi-selects can materialize joined models. The compact entity format
   used in this repository remains supported, so the migration does not require a
   wholesale switch to SeaORM's new dense model format. [Multi-selects](https://www.sea-ql.org/SeaORM/docs/relation/multi-selects/),
   [complex relations](https://www.sea-ql.org/SeaORM/docs/relation/complex-relations/)
5. `OnConflict` and `TryInsert` support idempotent inserts and conflict updates without
   hand-written `ON CONFLICT` clauses. [Insert API](https://docs.rs/sea-orm/2.0.2/sea_orm/query/struct.Insert.html),
   [TryInsert](https://docs.rs/sea-orm/2.0.2/sea_orm/query/struct.TryInsert.html)
6. Entity updates support `update_many`, typed expressions and affected-row counts;
   these are suitable for optimistic revision checks and guarded state transitions.
   [Update guide](https://www.sea-ql.org/SeaORM/docs/basic-crud/update/)
7. `lock_exclusive` and `lock_with_behavior` preserve `FOR UPDATE` and
   `SKIP LOCKED` semantics. Transactions can be explicit (`begin`/`commit`) or closure
   based. [QuerySelect locks](https://docs.rs/sea-orm/2.0.2/sea_orm/query/trait.QuerySelect.html),
   [transaction guide](https://www.sea-ql.org/SeaORM/docs/advanced-query/transaction/)
8. Aggregates are available through typed column methods such as `count`, `sum`,
   `group_by` and `having`. For the commercial summary, multiple typed aggregate queries
   or a typed partial projection are preferable to a PostgreSQL `FILTER` string.
   [Aggregate functions](https://www.sea-ql.org/SeaORM/docs/advanced-query/aggregate-function/)
9. SeaORM's migration API exposes `SchemaManager` plus SeaQuery builders for create,
   alter, drop, index, foreign-key and PostgreSQL type operations; its migration guide
   also documents seeding through SeaORM. This is the typed replacement for the 15
   migration files' `execute_unprepared` blocks.
   [Writing migrations](https://www.sea-ql.org/SeaORM/docs/migration/writing-migration/)

## Design conclusions

### Prefer entity APIs; allow typed SeaQuery expressions narrowly

The goal is compile-time schema and result-shape checking. Entity columns, relations,
active models and partial models provide that. SeaORM itself exposes SeaQuery expressions
for values, aggregates and locks; using `Expr::value` or typed column expressions remains
inside the ORM pipeline. Raw strings (`Expr::cust`, `raw_sql!`, `Statement`,
`query_*_raw`, `execute_raw`) should be forbidden in runtime code.

### Compose complex reads in Rust where it improves type safety

`run_history` currently uses a CTE/union because canonical runs and phone trials have
different row shapes. Two bounded entity queries, followed by a deterministic merge on
`(created_at, id)`, retain semantics while keeping both inputs typed. Likewise, JSON
provider matching in the model catalog can decode the bounded active definition set and
filter the contract type in Rust instead of embedding JSON path strings.

### Preserve database concurrency and invariant boundaries

The rewrite must retain transaction duration, lock order, conflict handling and affected
row assertions. No transaction may extend across device or network I/O. Unique indexes,
foreign keys and check constraints remain authoritative; ORM types complement rather than
replace them.

### Convert historical migrations with migration-safe typed builders

The raw runtime layer conflicts with `docs/architect/system.md:70-79`; removing it brings
implementation back to the documented architecture. Applied installations do not rerun
old migrations, but fresh installations and rollback/reapply tests do, so conversion must
preserve exact historical transitions. Use migration-local `DeriveIden` identifiers,
`SchemaManager`, and SeaQuery builders rather than current entities, whose shape may not
match the schema at that migration version. Zero raw SQL in all 15 files is required.

## Key risks

- Silent semantic drift in tuple pagination, history ordering or baseline eligibility.
- Lost `FOR UPDATE`/`SKIP LOCKED` behavior or a changed lock order causing races/deadlocks.
- Incorrect `ON CONFLICT` target or affected-row interpretation breaking idempotency.
- N+1 queries or unbounded in-memory composition replacing one complex SQL query.
- Entity field nullability/default mismatches with the post-migration schema.
- Rewriting test setup before all required entities exist, leaving broad integration
  failures that are hard to localize.
- Historical migration rewrites drifting in checks, indexes, data backfills, or down-order.

Mitigate with phased conversion, SQL-shape-independent route/database assertions,
concurrency tests, bounded fetch limits, exact post-migration entity definitions, and a
final source guard that prevents the raw escape hatches from returning.

## Methodology

The audit searched all Rust, migration, test, script and infrastructure sources for raw
SeaORM APIs and SQL DML/DDL tokens, traced every production caller of the central helper,
compared referenced tables with the entity module list, inspected the authoritative
architecture and migration files, and checked existing typed ORM examples. Graphify was
consulted but its index was older than `HEAD`, so every result was verified in source.

Exa MCP was invoked with three SeaORM research queries, but the configured free endpoint
returned a rate-limit error before any result. No credential or user configuration was
changed. Official SeaORM documentation, exact-version docs.rs pages and the SeaQL
repository/release materials were used as the fallback primary sources.
