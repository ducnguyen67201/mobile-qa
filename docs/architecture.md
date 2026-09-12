# Foundation boundaries

The root is a Loco 1.1 application; `migration/` uses SeaORM 2. Empty source registries
preserve generator integration points without inventing domain objects. PostgreSQL
contains only migration bookkeeping. Health is application liveness, not DB readiness.

`crates/contracts` is the sole transport source. The explicit exporter builds this
small crate, exports TS with ts-rs, and exports schema with Schemars. The coordinator
runs pinned datamodel-code-generator for Pydantic v2. It stages output independently
of the caller's working directory, writes only differing content, deletes obsolete
managed bindings, and compares additions/deletions/changes under `--check`.
There are no build scripts or test-side-effect exporters. Generated Python style is
excluded from lint; generated types still participate in consumer checking/tests.

`ContractProbe` demonstrates a tagged scenario, UUID and UTC-normalized RFC3339 time.
`nullable_note` is required and accepts string/null. `optional_note` accepts missing,
null or string; Rust omits it when null. Pydantic consumers use `exclude_unset=True`
when preserving omitted inputs, and must explicitly omit None to match Rust's
optional-null output normalization. `counter` is an arbitrary precision nonnegative
canonical decimal string (no sign or leading zeros), never a JavaScript number.
Rust callers invoke `validate()` after deserialization for semantic constraints;
Pydantic JSON parsing is strict and enforces schema constraints.

The fixture request has version 1, run UUID and pass/fail/blocked scenario. The result
reports passed/failed/blocked plus a deterministic observation. This is a local
fixture protocol, not the production claim/lease/result API of spec 04. Fake mode
never imports Minitap. The optional SDK command imports the pinned distribution and
checks Agent export; it cannot qualify device, evidence or execution behavior.

The frontend uses React Router and TanStack Query, a runtime-narrowing API helper,
and a small shadcn-derived Card/Button layout. Only App calls health. The API reserves
`/api/*` for real handlers/404, keeping unknown API URLs out of production SPA fallback.
Development/test disable static middleware. Production requires built `frontend/dist`
and explicit DATABASE_URL/HOST; it is not a deployment/security implementation.

Future Loco-generated DTOs move to the contract crate and are re-exported from
`src/dtos/`; remove generator `#[ts(export)]` attributes and register exports in the
explicit binary. Do not create handwritten equivalent browser/Python shapes.
Future migrations register above `inject-above`; route additions retain scaffold
markers. Specs 02 and 03 may build on this baseline independently with shared-file
ownership coordinated first.
