# Graph Report - mobile-qa  (2026-09-13)

## Corpus Check
- cluster-only mode — file stats not available

## Summary
- 2070 nodes · 5742 edges · 133 communities (88 shown, 29 thin omitted)
- Extraction: 93% EXTRACTED · 7% INFERRED · 0% AMBIGUOUS · INFERRED: 384 edges (avg confidence: 0.87)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `16c48e07`
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
- Community 64
- Community 65
- Community 66
- Community 67
- Community 68
- Community 69
- Community 70
- Community 71
- Community 72
- Community 73
- Community 74
- Community 75
- Community 76
- Community 77
- Community 78
- Community 79
- Community 80
- Community 81
- Community 82
- Community 83
- Community 84
- Community 85
- Community 86
- Community 87
- Community 88
- Community 90
- Community 91
- Community 92
- Community 93
- Community 94
- Community 95
- Community 96
- Community 97
- Community 98
- Community 99
- Community 100
- Community 101
- Community 102
- Community 103
- Community 104
- Community 105
- Community 106
- Community 107
- Community 108
- Community 110
- Community 111
- Community 112
- Community 113
- Community 114
- Community 115
- Community 116
- Community 117
- Community 132

## God Nodes (most connected - your core abstractions)
1. `QualificationError` - 116 edges
2. `field()` - 53 edges
3. `one()` - 53 edges
4. `useWorkspace()` - 52 edges
5. `checked()` - 49 edges
6. `Session` - 42 edges
7. `@mantine/core` - 40 edges
8. `Profile` - 38 edges
9. `Device` - 37 edges
10. `exec()` - 37 edges

## Surprising Connections (you probably didn't know these)
- `publish()` --indirect_call--> `task()`  [INFERRED]
  apps/mobile-worker/src/mobile_qa_worker/authoring/generation.py → scripts/execution_smoke.py
- `create()` --indirect_call--> `profile()`  [INFERRED]
  scripts/test_library_smoke.py → apps/mobile-worker/tests/test_device.py
- `target()` --references--> `DirectTarget`  [EXTRACTED]
  apps/api/src/services/test_authoring.rs → crates/contracts/src/automation.rs
- `reserve()` --calls--> `bounded()`  [INFERRED]
  apps/api/src/services/run_artifacts.rs → crates/contracts/src/execution.rs
- `create()` --calls--> `bounded()`  [INFERRED]
  apps/api/src/services/runs.rs → crates/contracts/src/execution.rs

## Import Cycles
- None detected.

## Communities (133 total, 29 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.05
Nodes (98): build(), claim(), detail(), open(), options(), ApiResult, AppContext, HeaderMap (+90 more)

### Community 1 - "Community 1"
Cohesion: 0.07
Nodes (92): build(), BuildQuery, claim(), cleanup(), complete(), events(), heartbeat(), reserve() (+84 more)

### Community 2 - "Community 2"
Cohesion: 0.06
Nodes (47): client, root, profile, second, router, routes, binaryUpload, completeStatuses (+39 more)

### Community 3 - "Community 3"
Cohesion: 0.07
Nodes (62): every_declared_route_requires_the_declared_security_and_errors(), google_identity_binding_expiry_replay_and_password_removal(), persisted_workflow_auth_scope_and_real_validation(), public_registration_approval_and_multiple_workspace_isolation(), rejection_recovery_and_infrastructure_failure(), throttles_quota_and_revision_scope(), approved(), definition() (+54 more)

### Community 4 - "Community 4"
Cohesion: 0.10
Nodes (62): artifact(), cancel(), create(), detail(), list(), ListQuery, preview(), ApiResult (+54 more)

### Community 5 - "Community 5"
Cohesion: 0.06
Nodes (41): Evidence, Path, safe_path(), validate_result(), One-device operator qualification; no production scheduler or customer…, test_campaign_oracle_never_reaches_runner(), run(), test_campaign_schedule_and_accounting() (+33 more)

### Community 6 - "Community 6"
Cohesion: 0.06
Nodes (44): configure_auth(), MAX_APK, AppContext, Config, Environment, Option, PathBuf, Result (+36 more)

### Community 7 - "Community 7"
Cohesion: 0.11
Nodes (48): execute(), Path, Sequential approved actions on the qualified demo adapter, with supervised SDK…, sdk_action(), accepted(), backend(), owned_backend(), Path (+40 more)

### Community 8 - "Community 8"
Cohesion: 0.08
Nodes (40): ApiError, ApkMetadata, AppListResponse, AppResponse, ApprovalStatus, AppSummary, BrowserApi, BuildListResponse (+32 more)

### Community 9 - "Community 9"
Cohesion: 0.07
Nodes (41): Path, Isolated structured model calls. This process has no worker identity or device…, run(), compact(), main(), Explicit developer commands, not a long-running production worker. `fake` emits…, Verify the pinned SDK import seam without constructing its stateful Agent.…, sdk_import() (+33 more)

### Community 10 - "Community 10"
Cohesion: 0.08
Nodes (31): cost_estimate(), rate(), Exception, QualificationError, Stable, safe reason code; never contains a provider response or credential., assert_ports_available(), Device, Path (+23 more)

### Community 11 - "Community 11"
Cohesion: 0.12
Nodes (41): Mutation, DefinitionKind, ArchiveLibraryEntryRequest, CreateLibraryEntryRequest, DefaultPlanResponse, ExecutionPlanQuery, ForkLibraryDraftRequest, LibraryCapabilities (+33 more)

### Community 12 - "Community 12"
Cohesion: 0.12
Nodes (39): b64(), client(), google_fixture(), google_sign_in(), main(), provision(), Secret-free HTTP acceptance using owned test accounts/processes and a synthetic…, Ephemeral RSA fixture keys; the API accepts these only in Environment::Test. (+31 more)

### Community 13 - "Community 13"
Cohesion: 0.13
Nodes (30): appQuery(), AccountMenu(), Navigation(), NavigationItem(), settingsLink, RunPreview(), Protected(), SessionContext (+22 more)

### Community 14 - "Community 14"
Cohesion: 0.16
Nodes (34): archiveLibraryEntry(), createLibraryEntry(), defaultPlanQuery(), forkLibraryDraft(), libraryDraftQuery(), libraryEntryQuery(), libraryErrorDetails(), libraryHistoryQuery() (+26 more)

### Community 15 - "Community 15"
Cohesion: 0.10
Nodes (30): allowed(), invoke(), Path, Bounded discovery drives the same direct commands testers record and rerun., Mask password regions before any provider request, including screen pixels., redact(), run(), ask() (+22 more)

### Community 16 - "Community 16"
Cohesion: 0.15
Nodes (29): archive(), archive_crc_integrity_and_abi_inventory(), attr(), checksum_mismatch_is_infrastructure_not_invalid_apk(), directory(), inspect(), inspect_archive(), Inspection (+21 more)

### Community 17 - "Community 17"
Cohesion: 0.18
Nodes (19): appsQuery(), App(), AppForm(), parseOrigins(), EnvironmentForm(), ErrorNotice(), ErrorNoticeProps, LoadingPanel() (+11 more)

### Community 18 - "Community 18"
Cohesion: 0.16
Nodes (27): claim(), claim_wait(), cleanup(), complete(), events(), heartbeat(), lease(), reconcile() (+19 more)

### Community 19 - "Community 19"
Cohesion: 0.10
Nodes (17): AppRoutes, App, AppContext, Box, Config, Environment, Path, Result (+9 more)

### Community 20 - "Community 20"
Cohesion: 0.29
Nodes (28): complete_upload(), create_app(), create_upload(), create_workspace(), get_app(), get_build(), get_upload(), google_challenge() (+20 more)

### Community 21 - "Community 21"
Cohesion: 0.11
Nodes (26): evaluate(), observe(), png_valid(), ApiResult, AppContext, Result, String, Uuid (+18 more)

### Community 22 - "Community 22"
Cohesion: 0.21
Nodes (24): cancelRun(), createRun(), planQuery(), runQuery(), runsQuery(), checked(), forgetSession(), headers() (+16 more)

### Community 23 - "Community 23"
Cohesion: 0.17
Nodes (16): ArtifactStore, private_immutable_publication_and_symlink_rejection(), ApiResult, DateTime, File, Option, Path, PathBuf (+8 more)

### Community 24 - "Community 24"
Cohesion: 0.10
Nodes (17): execute(), png(), Path, Synthetic HTTP-worker evidence. It is never advertised as device qualification., job(), parametrize, Offline transport and action evidence tests; never boot or call a model., test_checkpoint_deadline_respects_approved_window() (+9 more)

### Community 25 - "Community 25"
Cohesion: 0.19
Nodes (20): phoneOptionsQuery(), phoneQuery(), saveAuthoredTests(), PhoneWorkspace(), ResizableWorkspace(), commandTarget(), newCheck(), newTaskStep() (+12 more)

### Community 26 - "Community 26"
Cohesion: 0.18
Nodes (22): decode(), exec(), hash(), json(), ApiResult, String, T, Value (+14 more)

### Community 27 - "Community 27"
Cohesion: 0.15
Nodes (16): Path, One sequence, shared device executor, durable per-step receipts., run(), act(), capture(), controls(), goal_for(), Explicit interactive device worker; ordinary checks never import or invoke… (+8 more)

### Community 28 - "Community 28"
Cohesion: 0.20
Nodes (17): buildQuery(), buildsQuery(), completeUpload(), getUpload(), ApkUpload(), BuildDetail(), colors, explanations (+9 more)

### Community 29 - "Community 29"
Cohesion: 0.32
Nodes (20): authorized(), create(), detail(), environment(), list(), ListQuery, membership(), origins() (+12 more)

### Community 30 - "Community 30"
Cohesion: 0.35
Nodes (20): admitted(), authorize(), capabilities(), content_issues(), coverage(), default_plan(), draft(), entry() (+12 more)

### Community 31 - "Community 31"
Cohesion: 0.10
Nodes (19): name, packageManager, private, type, version, eslint, @eslint/js, globals (+11 more)

### Community 32 - "Community 32"
Cohesion: 0.10
Nodes (20): compilerOptions, allowImportingTsExtensions, baseUrl, esModuleInterop, jsx, lib, module, moduleResolution (+12 more)

### Community 33 - "Community 33"
Cohesion: 0.43
Nodes (19): field(), authorized(), claim(), detail(), lease(), open(), options(), read() (+11 more)

### Community 34 - "Community 34"
Cohesion: 0.30
Nodes (19): one(), rows(), ConnectionTrait, QueryResult, Vec, attempt(), authorize(), cancel() (+11 more)

### Community 35 - "Community 35"
Cohesion: 0.16
Nodes (13): execute(), parse_request(), Return the result chosen by a fixture; do not discover whether an app works.…, Validate fixture JSON without coercing values such as string versions to…, Echo the run ID and map the requested scenario to its fixed simulated outcome.…, parametrize, Python half of the shared fixture contract and local CLI checks. No device SDK…, test_cli_invalid_nonzero() (+5 more)

### Community 36 - "Community 36"
Cohesion: 0.11
Nodes (18): devDependencies, eslint, @eslint/js, globals, @hey-api/openapi-ts, jsdom, prettier, @testing-library/jest-dom (+10 more)

### Community 37 - "Community 37"
Cohesion: 0.24
Nodes (14): arg(), execute(), Execution, id(), purpose(), read(), ApiResult, AppContext (+6 more)

### Community 39 - "Community 39"
Cohesion: 0.33
Nodes (15): action(), cancel(), command(), enqueue(), generate(), job_detail(), ApiResult, AppContext (+7 more)

### Community 40 - "Community 40"
Cohesion: 0.42
Nodes (15): build(), build_response(), complete(), create(), list(), ApiResult, AppContext, ListQuery (+7 more)

### Community 41 - "Community 41"
Cohesion: 0.23
Nodes (13): arg(), execute(), id(), Operator, ApiResult, AppContext, Result, String (+5 more)

### Community 42 - "Community 42"
Cohesion: 0.18
Nodes (10): Explicit bounded navigation; imports the SDK after process environment…, execute(), navigate(), prepare_environment(), Path, Only this subprocess imports Minitap. Its output never decides the QA verdict., Shared pinned SDK seam; the caller owns the authorized navigation instruction., install_tool_runtime_compat() (+2 more)

### Community 43 - "Community 43"
Cohesion: 0.13
Nodes (3): UsageRecorder, test_sdk_exact_public_seam(), test_usage_deduplicates_and_keeps_unknown()

### Community 44 - "Community 44"
Cohesion: 0.38
Nodes (11): conflict(), apply(), bump(), link_import(), review(), ApiResult, AppContext, ConnectionTrait (+3 more)

### Community 45 - "Community 45"
Cohesion: 0.32
Nodes (6): android.app.Activity, android.os.Bundle, android.widget.LinearLayout, MainActivity, LinearLayout, Override

### Community 46 - "Community 46"
Cohesion: 0.26
Nodes (9): files(), main(), Path, Generate both consumer contracts from Rust without starting the application.…, Read managed output bytes keyed by repo-relative path, independent of mtimes., Compare the whole generated set, including new and removed files. Preserve…, synchronize(), ExportTests (+1 more)

### Community 47 - "Community 47"
Cohesion: 0.22
Nodes (8): Cleanup, execute(), ApiResult, AppContext, Result, Task, TaskInfo, Vars

### Community 48 - "Community 48"
Cohesion: 0.18
Nodes (11): dependencies, lucide-react, @mantine/core, @mantine/form, @mantine/hooks, react, react-dom, @react-oauth/google (+3 more)

### Community 49 - "Community 49"
Cohesion: 0.24
Nodes (8): ExecutionWakeup, notify(), AppContext, Self, subscribe(), Default, Receiver, Sender

### Community 50 - "Community 50"
Cohesion: 0.31
Nodes (6): healthQuery, apiClient, ApiClientError, requestMethods, transportClient, healthy

### Community 51 - "Community 51"
Cohesion: 0.31
Nodes (7): current(), method_not_allowed(), Json, StatusCode, unknown(), HealthResponse, HealthStatus

### Community 52 - "Community 52"
Cohesion: 0.25
Nodes (3): DeviceRpc, ElementRpc, Protocol

### Community 53 - "Community 53"
Cohesion: 0.22
Nodes (9): scripts, build, dev, format, format:check, generate:sdk, lint, test (+1 more)

### Community 56 - "Community 56"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 57 - "Community 57"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 58 - "Community 58"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 59 - "Community 59"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 60 - "Community 60"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 61 - "Community 61"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 62 - "Community 62"
Cohesion: 0.25
Nodes (7): Model, Relation, DateTimeUtc, Json, Option, String, Uuid

### Community 63 - "Community 63"
Cohesion: 0.25
Nodes (7): Model, Relation, DateTimeUtc, Json, Option, String, Uuid

### Community 64 - "Community 64"
Cohesion: 0.36
Nodes (5): installer(), parametrize, Exercise the pinned installer with local archives only; never contact Google., test_install_checks_archive_and_preserves_managed_metadata(), test_installer_only_selects_native_system_image()

### Community 65 - "Community 65"
Cohesion: 0.54
Nodes (5): startGoogleSignIn(), useGoogleSignIn(), safeReturnTo(), SignIn(), @react-oauth/google

### Community 67 - "Community 67"
Cohesion: 0.29
Nodes (5): Migrator, Box, MigrationTrait, Vec, MigratorTrait

### Community 68 - "Community 68"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 69 - "Community 69"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 70 - "Community 70"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 71 - "Community 71"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 72 - "Community 72"
Cohesion: 0.48
Nodes (6): fixture(), optional_null_is_accepted_then_omitted(), probe_roundtrips_without_losing_precision(), rejects_invalid_probe_boundaries(), Value, scenario_and_version_are_validated()

### Community 73 - "Community 73"
Cohesion: 0.48
Nodes (5): approved_import_shape_has_separate_actions_and_checks(), case(), invalid_definition_references_and_paths_are_rejected(), manual_methods_are_defined_but_not_implicitly_executable(), unknown_fields_are_not_imported()

### Community 74 - "Community 74"
Cohesion: 0.43
Nodes (4): case(), drafts_round_trip_and_publication_preserves_the_execution_shape(), incomplete_case_is_saveable_but_not_publishable(), oversized_unknown_and_forged_shapes_are_rejected()

### Community 75 - "Community 75"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 76 - "Community 76"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 77 - "Community 77"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 78 - "Community 78"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 79 - "Community 79"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 80 - "Community 80"
Cohesion: 0.47
Nodes (5): create(), ApiResult, AppContext, Uuid, CreateWorkspaceRequest

### Community 81 - "Community 81"
Cohesion: 0.40
Nodes (4): request_context(), Response, Next, Request

### Community 82 - "Community 82"
Cohesion: 0.40
Nodes (4): Model, Relation, String, Uuid

### Community 83 - "Community 83"
Cohesion: 0.50
Nodes (3): Protocol, StructuredCall, StructuredModel

### Community 84 - "Community 84"
Cohesion: 0.40
Nodes (4): main(), Box, Error, Result

### Community 85 - "Community 85"
Cohesion: 0.50
Nodes (3): rejects_invalid_request_semantics(), request(), Value

### Community 86 - "Community 86"
Cohesion: 0.70
Nodes (4): java_environment(), main(), Explicit local SDK setup and demo build; never runs checks or accepts SDK…, setup()

### Community 87 - "Community 87"
Cohesion: 0.50
Nodes (3): Model, Relation, Uuid

### Community 88 - "Community 88"
Cohesion: 0.83
Nodes (3): gradlew script, die(), warn()

### Community 91 - "Community 91"
Cohesion: 0.67
Nodes (3): main(), Explicit repository formatting; generators retain ownership of generated output., run()

### Community 92 - "Community 92"
Cohesion: 0.67
Nodes (3): install(), main(), Explicit project-local Android intake tool installation, never run at API…

### Community 94 - "Community 94"
Cohesion: 0.67
Nodes (3): migration, mobile-qa, mobile-qa-contracts

## Knowledge Gaps
- **144 isolated node(s):** `ErrorNoticeProps`, `PageHeadingProps`, `ActiveModel`, `ActiveModel`, `ActiveModel` (+139 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 619 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **29 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Session` connect `Community 4` to `Community 0`, `Community 40`, `Community 6`?**
  _High betweenness centrality (0.037) - this node is a cross-community bridge._
- **Why does `ApiFailure` connect `Community 6` to `Community 18`, `Community 44`, `Community 30`?**
  _High betweenness centrality (0.023) - this node is a cross-community bridge._
- **Why does `task_session_requires_no_plan_and_fences_worker_and_task_identity()` connect `Community 3` to `Community 26`, `Community 34`, `Community 23`?**
  _High betweenness centrality (0.017) - this node is a cross-community bridge._
- **Are the 36 inferred relationships involving `QualificationError` (e.g. with `owned_backend()` and `Device`) actually correct?**
  _`QualificationError` has 36 INFERRED edges - model-reasoned connections that need verification._
- **Are the 49 inferred relationships involving `field()` (e.g. with `build()` and `cleanup_pending()`) actually correct?**
  _`field()` has 49 INFERRED edges - model-reasoned connections that need verification._
- **Are the 47 inferred relationships involving `one()` (e.g. with `artifact()` and `build()`) actually correct?**
  _`one()` has 47 INFERRED edges - model-reasoned connections that need verification._
- **What connects `ErrorNoticeProps`, `PageHeadingProps`, `ActiveModel` to the rest of the system?**
  _144 weakly-connected nodes found - possible documentation gaps or missing edges._