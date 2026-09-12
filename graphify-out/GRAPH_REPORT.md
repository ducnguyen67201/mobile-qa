# Graph Report - mobile-qa  (2026-09-12)

## Corpus Check
- cluster-only mode — file stats not available

## Summary
- 901 nodes · 1904 edges · 76 communities (44 shown, 21 thin omitted)
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS · INFERRED: 8 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `72bd3fbe`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Community 0
- Community 1
- Community 2
- Community 3
- Community 4
- Community 5
- Community 6
- Community 7
- Community 8
- Community 9
- Community 10
- Community 11
- Community 12
- Community 13
- Community 14
- Community 15
- Community 16
- Community 17
- Community 18
- Community 19
- Community 20
- Community 21
- Community 22
- Community 23
- Community 24
- Community 25
- Community 26
- Community 27
- Community 28
- Community 29
- Community 30
- Community 31
- Community 32
- Community 33
- Community 34
- Community 35
- Community 36
- Community 37
- Community 38
- Community 39
- Community 40
- Community 41
- Community 42
- Community 43
- Community 44
- Community 45
- Community 46
- Community 47
- Community 48
- Community 49
- Community 50
- Community 51
- Community 52
- Community 53
- Community 54
- Community 55
- Community 56
- Community 57
- Community 58
- Community 59
- Community 60
- Community 61
- Community 62
- Community 63
- Community 75

## God Nodes (most connected - your core abstractions)
1. `@mantine/core` - 23 edges
2. `useWorkspace()` - 19 edges
3. `compilerOptions` - 19 edges
4. `session()` - 19 edges
5. `react-router` - 18 edges
6. `checked()` - 17 edges
7. `@tanstack/react-query` - 17 edges
8. `ApiFailure` - 16 edges
9. `lucide-react` - 15 edges
10. `Setup` - 15 edges

## Surprising Connections (you probably didn't know these)
- `logout()` --references--> `LogoutResponse`  [EXTRACTED]
  apps/api/src/controllers/setup.rs → crates/contracts/src/browser.rs
- `Inspection` --references--> `ApkMetadata`  [EXTRACTED]
  apps/api/src/services/apk_validation.rs → crates/contracts/src/browser.rs
- `metadata()` --references--> `ApkMetadata`  [EXTRACTED]
  apps/api/src/services/apk_validation.rs → crates/contracts/src/browser.rs
- `list_apps()` --references--> `AppListResponse`  [EXTRACTED]
  apps/api/src/controllers/setup.rs → crates/contracts/src/browser.rs
- `list()` --references--> `AppListResponse`  [EXTRACTED]
  apps/api/src/services/apps.rs → crates/contracts/src/browser.rs

## Import Cycles
- None detected.

## Communities (76 total, 21 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.06
Nodes (84): healthQuery, apiClient, ApiClientError, requestMethods, transportClient, appQuery(), appsQuery(), buildQuery() (+76 more)

### Community 1 - "Community 1"
Cohesion: 0.05
Nodes (56): current(), method_not_allowed(), Json, StatusCode, unknown(), request_context(), Response, create() (+48 more)

### Community 2 - "Community 2"
Cohesion: 0.07
Nodes (29): second, routes, binaryUpload, completeStatuses, healthy, persistedBuild, staleBuild, staleResponse (+21 more)

### Community 3 - "Community 3"
Cohesion: 0.09
Nodes (33): configure_auth(), MAX_APK, AppContext, Config, Environment, Option, PathBuf, Result (+25 more)

### Community 4 - "Community 4"
Cohesion: 0.15
Nodes (31): challenge(), configure_google(), create_input(), DATABASE_BOOT, error(), every_declared_route_requires_the_declared_security_and_errors(), fixture(), GOOGLE_CLIENT (+23 more)

### Community 5 - "Community 5"
Cohesion: 0.15
Nodes (29): archive(), archive_crc_integrity_and_abi_inventory(), attr(), checksum_mismatch_is_infrastructure_not_invalid_apk(), directory(), inspect(), inspect_archive(), Inspection (+21 more)

### Community 6 - "Community 6"
Cohesion: 0.10
Nodes (17): AppRoutes, App, AppContext, Box, Config, Environment, Path, Result (+9 more)

### Community 7 - "Community 7"
Cohesion: 0.30
Nodes (27): complete_upload(), create_app(), create_upload(), create_workspace(), get_app(), get_build(), get_upload(), google_challenge() (+19 more)

### Community 8 - "Community 8"
Cohesion: 0.17
Nodes (16): ArtifactStore, private_immutable_publication_and_symlink_rejection(), ApiResult, DateTime, File, Option, Path, PathBuf (+8 more)

### Community 9 - "Community 9"
Cohesion: 0.12
Nodes (24): ContractProbe, FakeExecutionRequest, FakeExecutionResult, nullable_string_schema(), Outcome, required_nullable(), DateTime, Error (+16 more)

### Community 10 - "Community 10"
Cohesion: 0.12
Nodes (18): main(), Explicit developer commands, not a long-running production worker. `fake` emits…, Verify the pinned SDK import seam without constructing its stateful Agent.…, sdk_import(), execute(), parse_request(), Return the result chosen by a fixture; do not discover whether an app works.…, Validate fixture JSON without coercing values such as string versions to… (+10 more)

### Community 11 - "Community 11"
Cohesion: 0.21
Nodes (22): authenticate(), ensure_live(), issue_session(), LoginGuard, organization_memberships(), origin(), provision(), rate_limit() (+14 more)

### Community 12 - "Community 12"
Cohesion: 0.16
Nodes (23): b64(), client(), google_fixture(), google_sign_in(), main(), provision(), Secret-free HTTP acceptance using owned test accounts/processes and a synthetic…, Ephemeral RSA fixture keys; the API accepts these only in Environment::Test. (+15 more)

### Community 13 - "Community 13"
Cohesion: 0.32
Nodes (20): authorized(), create(), detail(), environment(), list(), ListQuery, membership(), origins() (+12 more)

### Community 14 - "Community 14"
Cohesion: 0.10
Nodes (20): compilerOptions, allowImportingTsExtensions, baseUrl, esModuleInterop, jsx, lib, module, moduleResolution (+12 more)

### Community 15 - "Community 15"
Cohesion: 0.11
Nodes (18): name, packageManager, private, type, version, eslint, @eslint/js, globals (+10 more)

### Community 16 - "Community 16"
Cohesion: 0.22
Nodes (10): ApiFailure, DbErr, Error, Response, Self, StatusCode, String, From (+2 more)

### Community 17 - "Community 17"
Cohesion: 0.12
Nodes (17): devDependencies, eslint, @eslint/js, globals, @hey-api/openapi-ts, jsdom, @testing-library/jest-dom, @testing-library/react (+9 more)

### Community 18 - "Community 18"
Cohesion: 0.23
Nodes (13): arg(), execute(), id(), Operator, ApiResult, AppContext, Result, String (+5 more)

### Community 19 - "Community 19"
Cohesion: 0.48
Nodes (14): build(), build_response(), complete(), create(), list(), ApiResult, AppContext, Model (+6 more)

### Community 20 - "Community 20"
Cohesion: 0.26
Nodes (9): files(), main(), Path, Generate both consumer contracts from Rust without starting the application.…, Read managed output bytes keyed by repo-relative path, independent of mtimes., Compare the whole generated set, including new and removed files. Preserve…, synchronize(), ExportTests (+1 more)

### Community 21 - "Community 21"
Cohesion: 0.22
Nodes (8): Cleanup, execute(), ApiResult, AppContext, Result, Task, TaskInfo, Vars

### Community 22 - "Community 22"
Cohesion: 0.18
Nodes (11): dependencies, lucide-react, @mantine/core, @mantine/form, @mantine/hooks, react, react-dom, @react-oauth/google (+3 more)

### Community 23 - "Community 23"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 24 - "Community 24"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 25 - "Community 25"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 26 - "Community 26"
Cohesion: 0.25
Nodes (7): Model, Relation, DateTimeUtc, Json, Option, String, Uuid

### Community 27 - "Community 27"
Cohesion: 0.25
Nodes (7): Model, Relation, DateTimeUtc, Json, Option, String, Uuid

### Community 28 - "Community 28"
Cohesion: 0.29
Nodes (5): Migrator, Box, MigrationTrait, Vec, MigratorTrait

### Community 29 - "Community 29"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 30 - "Community 30"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 31 - "Community 31"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 32 - "Community 32"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 33 - "Community 33"
Cohesion: 0.29
Nodes (7): scripts, build, dev, generate:sdk, lint, test, typecheck

### Community 34 - "Community 34"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 35 - "Community 35"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 36 - "Community 36"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 37 - "Community 37"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 38 - "Community 38"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 39 - "Community 39"
Cohesion: 0.40
Nodes (4): Model, Relation, String, Uuid

### Community 40 - "Community 40"
Cohesion: 0.40
Nodes (4): main(), Box, Error, Result

### Community 41 - "Community 41"
Cohesion: 0.50
Nodes (3): Model, Relation, Uuid

### Community 43 - "Community 43"
Cohesion: 0.67
Nodes (3): install(), main(), Explicit project-local Android intake tool installation, never run at API…

### Community 45 - "Community 45"
Cohesion: 0.67
Nodes (3): migration, mobile-qa, mobile-qa-contracts

## Knowledge Gaps
- **128 isolated node(s):** `ErrorNoticeProps`, `PageHeadingProps`, `requestMethods`, `transportClient`, `options` (+123 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 355 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **21 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Setup` connect `Community 3` to `Community 8`, `Community 11`, `Community 5`?**
  _High betweenness centrality (0.028) - this node is a cross-community bridge._
- **Why does `ArtifactStore` connect `Community 8` to `Community 3`?**
  _High betweenness centrality (0.013) - this node is a cross-community bridge._
- **Why does `ApiFailure` connect `Community 16` to `Community 3`?**
  _High betweenness centrality (0.013) - this node is a cross-community bridge._
- **What connects `ErrorNoticeProps`, `PageHeadingProps`, `requestMethods` to the rest of the system?**
  _128 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.061857163552078806 - nodes in this community are weakly interconnected._
- **Should `Community 1` be split into smaller, more focused modules?**
  _Cohesion score 0.05389942788316772 - nodes in this community are weakly interconnected._
- **Should `Community 2` be split into smaller, more focused modules?**
  _Cohesion score 0.06972789115646258 - nodes in this community are weakly interconnected._