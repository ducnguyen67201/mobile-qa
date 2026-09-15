# Graph Report - mobile-qa  (2026-09-15)

## Corpus Check
- cluster-only mode — file stats not available

## Summary
- 2197 nodes · 6068 edges · 132 communities (88 shown, 28 thin omitted)
- Extraction: 93% EXTRACTED · 7% INFERRED · 0% AMBIGUOUS · INFERRED: 415 edges (avg confidence: 0.88)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `680274a0`
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
- Community 86
- Community 87
- Community 88
- Community 89
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
- Community 131

## God Nodes (most connected - your core abstractions)
1. `QualificationError` - 141 edges
2. `field()` - 53 edges
3. `one()` - 53 edges
4. `useWorkspace()` - 52 edges
5. `checked()` - 50 edges
6. `@mantine/core` - 44 edges
7. `Profile` - 43 edges
8. `Session` - 42 edges
9. `Device` - 41 edges
10. `exec()` - 37 edges

## Surprising Connections (you probably didn't know these)
- `task()` --indirect_call--> `client()`  [INFERRED]
  apps/mobile-worker/src/mobile_qa_worker/authoring/minitap_discovery.py → scripts/app_setup_smoke.py
- `create()` --indirect_call--> `profile()`  [INFERRED]
  scripts/test_library_smoke.py → apps/mobile-worker/tests/test_device.py
- `logout()` --references--> `LogoutResponse`  [EXTRACTED]
  apps/api/src/controllers/setup.rs → crates/contracts/src/browser.rs
- `test_checkpoint_deadline_respects_approved_window()` --uses--> `QualificationError`  [INFERRED]
  apps/mobile-worker/tests/test_execution.py → apps/mobile-worker/src/mobile_qa_worker/qualification/config.py
- `cannot_stop()` --calls--> `QualificationError`  [INFERRED]
  apps/mobile-worker/tests/test_execution.py → apps/mobile-worker/src/mobile_qa_worker/qualification/config.py

## Import Cycles
- None detected.

## Communities (132 total, 28 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.06
Nodes (139): conflict(), decode(), exec(), field(), hash(), json(), one(), rows() (+131 more)

### Community 1 - "Community 1"
Cohesion: 0.06
Nodes (103): artifact(), cancel(), create(), detail(), list(), ListQuery, preview(), ApiResult (+95 more)

### Community 2 - "Community 2"
Cohesion: 0.06
Nodes (90): build(), BuildQuery, claim(), cleanup(), complete(), events(), heartbeat(), reserve() (+82 more)

### Community 3 - "Community 3"
Cohesion: 0.07
Nodes (62): every_declared_route_requires_the_declared_security_and_errors(), google_identity_binding_expiry_replay_and_password_removal(), persisted_workflow_auth_scope_and_real_validation(), public_registration_approval_and_multiple_workspace_isolation(), rejection_recovery_and_infrastructure_failure(), throttles_quota_and_revision_scope(), approved(), definition() (+54 more)

### Community 4 - "Community 4"
Cohesion: 0.07
Nodes (53): b64(), client(), google_fixture(), google_sign_in(), main(), provision(), Secret-free HTTP acceptance using owned test accounts/processes and a synthetic…, Ephemeral RSA fixture keys; the API accepts these only in Environment::Test. (+45 more)

### Community 5 - "Community 5"
Cohesion: 0.05
Nodes (45): Isolated structured model calls. This process has no worker identity or device…, materialize(), Materialize a model-selected journal prefix; models never manufacture replay…, Client, NoRedirect, Exception, T, Bounded, same-origin HTTP; generated Pydantic validates every JSON response. (+37 more)

### Community 6 - "Community 6"
Cohesion: 0.06
Nodes (40): host_environment(), Evidence, Path, Private immutable attempt evidence. A trace filename is never a QA verdict., safe_path(), validate_result(), One-device operator qualification; no production scheduler or customer…, test_campaign_oracle_never_reaches_runner() (+32 more)

### Community 7 - "Community 7"
Cohesion: 0.08
Nodes (40): ApiError, ApkMetadata, AppListResponse, AppResponse, ApprovalStatus, AppSummary, BrowserApi, BuildListResponse (+32 more)

### Community 8 - "Community 8"
Cohesion: 0.10
Nodes (34): session, proposal, profile, second, routes, apiError(), app, appId (+26 more)

### Community 9 - "Community 9"
Cohesion: 0.11
Nodes (42): runsQuery(), appQuery(), appsQuery(), buildQuery(), buildsQuery(), completeUpload(), createApp(), createUpload() (+34 more)

### Community 10 - "Community 10"
Cohesion: 0.08
Nodes (45): configure_auth(), MAX_APK, AppContext, Config, Environment, Option, PathBuf, Result (+37 more)

### Community 11 - "Community 11"
Cohesion: 0.05
Nodes (35): Any, Explicit bounded AI authoring; ordinary device operations never enter this…, check_context(), compact(), configure_agent(), __call__(), direct_tool(), foreground() (+27 more)

### Community 12 - "Community 12"
Cohesion: 0.09
Nodes (36): Parent-owned discovery effects. The SDK receives observations, never device…, Bounded discovery drives the same direct commands testers record and rerun., check(), execute(), hierarchy(), nodes(), Execute one typed command on the owned serial. Mutating calls are never retried., Trial assertions use retained hierarchy properties, independently of AI prose. (+28 more)

### Community 13 - "Community 13"
Cohesion: 0.13
Nodes (42): execute(), Path, Sequential approved actions on the qualified demo adapter, with supervised SDK…, sdk_action(), accepted(), backend(), owned_backend(), Path (+34 more)

### Community 14 - "Community 14"
Cohesion: 0.09
Nodes (31): DiscoveryBroker, explore(), Path, Bounded pipe protocol; SDK logging goes to /dev/null, never into public…, invoke(), Path, Mask password regions before any provider request, including screen pixels., redact() (+23 more)

### Community 15 - "Community 15"
Cohesion: 0.15
Nodes (39): archiveLibraryEntry(), createLibraryEntry(), defaultPlanQuery(), forkLibraryDraft(), libraryDraftQuery(), libraryEntryQuery(), libraryErrorDetails(), libraryHistoryQuery() (+31 more)

### Community 16 - "Community 16"
Cohesion: 0.07
Nodes (27): BrokerClient, Path, Pinned SDK graph adaptation. All device/model observations cross the parent…, run(), Path, run(), compact(), main() (+19 more)

### Community 17 - "Community 17"
Cohesion: 0.11
Nodes (28): ApiFailure, DbErr, Error, From, Into, Option, Response, Self (+20 more)

### Community 18 - "Community 18"
Cohesion: 0.17
Nodes (34): AuthoringModelRequest, AuthoringModelResponse, AuthoringUsage, AutomationSequence, CoverageKind, DirectCommand, DiscoveryCall, DiscoveryDecision (+26 more)

### Community 19 - "Community 19"
Cohesion: 0.13
Nodes (26): createWorkspace(), sessionQuery, signOut(), App(), AccountMenu(), Navigation(), NavigationItem(), navigationLinks (+18 more)

### Community 20 - "Community 20"
Cohesion: 0.18
Nodes (26): checked(), headers(), saveEnvironment(), openPhone(), phoneOptionsQuery(), phoneQuery(), stopPhone(), cancelTestGeneration() (+18 more)

### Community 21 - "Community 21"
Cohesion: 0.10
Nodes (17): AppRoutes, App, AppContext, Box, Config, Environment, Path, Result (+9 more)

### Community 22 - "Community 22"
Cohesion: 0.29
Nodes (28): complete_upload(), create_app(), create_upload(), create_workspace(), get_app(), get_build(), get_upload(), google_challenge() (+20 more)

### Community 23 - "Community 23"
Cohesion: 0.11
Nodes (26): evaluate(), observe(), png_valid(), ApiResult, AppContext, Result, String, Uuid (+18 more)

### Community 24 - "Community 24"
Cohesion: 0.08
Nodes (16): parametrize, test_full_supervisor_with_injected_device(), boot(), capture(), sdk(), test_demo_navigation_uses_current_control_bounds(), adb(), test_mac_boot_uses_effective_locale_and_unattended_flags() (+8 more)

### Community 25 - "Community 25"
Cohesion: 0.17
Nodes (16): ArtifactStore, private_immutable_publication_and_symlink_rejection(), ApiResult, DateTime, File, Option, Path, PathBuf (+8 more)

### Community 26 - "Community 26"
Cohesion: 0.32
Nodes (20): authorized(), create(), detail(), environment(), list(), ListQuery, membership(), origins() (+12 more)

### Community 27 - "Community 27"
Cohesion: 0.10
Nodes (19): name, packageManager, private, type, version, eslint, @eslint/js, globals (+11 more)

### Community 28 - "Community 28"
Cohesion: 0.09
Nodes (12): binaryUpload, completeStatuses, editableDefinition, healthy, implicitDefault, persistedBuild, staleBuild, staleResponse (+4 more)

### Community 29 - "Community 29"
Cohesion: 0.10
Nodes (20): compilerOptions, allowImportingTsExtensions, baseUrl, esModuleInterop, jsx, lib, module, moduleResolution (+12 more)

### Community 30 - "Community 30"
Cohesion: 0.16
Nodes (18): ContractProbe, FakeExecutionRequest, FakeExecutionResult, nullable_string_schema(), Outcome, required_nullable(), DateTime, Error (+10 more)

### Community 31 - "Community 31"
Cohesion: 0.31
Nodes (19): OpenPhoneRequest, PhoneBuildChoice, PhoneClaimRequest, PhoneClaimResponse, PhoneControl, PhoneFrame, PhoneLease, PhoneOptions (+11 more)

### Community 32 - "Community 32"
Cohesion: 0.37
Nodes (17): build(), claim(), detail(), open(), options(), ApiResult, AppContext, HeaderMap (+9 more)

### Community 33 - "Community 33"
Cohesion: 0.16
Nodes (13): execute(), parse_request(), Return the result chosen by a fixture; do not discover whether an app works.…, Validate fixture JSON without coercing values such as string versions to…, Echo the run ID and map the requested scenario to its fixed simulated outcome.…, parametrize, Python half of the shared fixture contract and local CLI checks. No device SDK…, test_cli_invalid_nonzero() (+5 more)

### Community 34 - "Community 34"
Cohesion: 0.11
Nodes (18): devDependencies, eslint, @eslint/js, globals, @hey-api/openapi-ts, jsdom, prettier, @testing-library/jest-dom (+10 more)

### Community 35 - "Community 35"
Cohesion: 0.25
Nodes (10): cancelRun(), createRun(), planQuery(), runQuery(), forgetSession(), options, setCsrfToken(), RunPreview() (+2 more)

### Community 36 - "Community 36"
Cohesion: 0.24
Nodes (14): arg(), execute(), Execution, id(), purpose(), read(), ApiResult, AppContext (+6 more)

### Community 37 - "Community 37"
Cohesion: 0.18
Nodes (10): Device, parametrize, Deterministic device seam; no emulator, network or model credentials., Rpc, test_cancel_before_action_has_no_effect(), test_capture_shares_active_rpc_instead_of_starting_a_second_instrumentation(), test_literal_set_text_uses_rpc_without_model(), test_rejects_ambiguous_disabled_secret_outside_targets() (+2 more)

### Community 39 - "Community 39"
Cohesion: 0.42
Nodes (15): build(), build_response(), complete(), create(), list(), ApiResult, AppContext, ListQuery (+7 more)

### Community 40 - "Community 40"
Cohesion: 0.23
Nodes (13): arg(), execute(), id(), Operator, ApiResult, AppContext, Result, String (+5 more)

### Community 41 - "Community 41"
Cohesion: 0.13
Nodes (3): UsageRecorder, test_sdk_exact_public_seam(), test_usage_deduplicates_and_keeps_unknown()

### Community 42 - "Community 42"
Cohesion: 0.25
Nodes (13): commandTarget(), newCheck(), newTaskStep(), sequenceReady(), TaskSteps(), BudgetFields(), moveItem(), Ordering() (+5 more)

### Community 43 - "Community 43"
Cohesion: 0.48
Nodes (12): cancel(), command(), detail(), generate(), ApiResult, AppContext, Json, Path (+4 more)

### Community 44 - "Community 44"
Cohesion: 0.32
Nodes (6): android.app.Activity, android.os.Bundle, android.widget.LinearLayout, MainActivity, LinearLayout, Override

### Community 45 - "Community 45"
Cohesion: 0.26
Nodes (6): healthQuery, apiClient, ApiClientError, requestMethods, transportClient, healthy

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
Nodes (7): current(), method_not_allowed(), Json, StatusCode, unknown(), HealthResponse, HealthStatus

### Community 51 - "Community 51"
Cohesion: 0.25
Nodes (3): DeviceRpc, ElementRpc, Protocol

### Community 52 - "Community 52"
Cohesion: 0.22
Nodes (9): scripts, build, dev, format, format:check, generate:sdk, lint, test (+1 more)

### Community 53 - "Community 53"
Cohesion: 0.47
Nodes (6): signIn(), startGoogleSignIn(), useGoogleSignIn(), safeReturnTo(), SignIn(), @react-oauth/google

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

### Community 66 - "Community 66"
Cohesion: 0.29
Nodes (5): Migrator, Box, MigrationTrait, Vec, MigratorTrait

### Community 67 - "Community 67"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

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
Cohesion: 0.43
Nodes (6): discovery_progress(), ApiResult, Option, T, same(), task_update()

### Community 72 - "Community 72"
Cohesion: 0.38
Nodes (3): target(), DirectTarget, Result

### Community 73 - "Community 73"
Cohesion: 0.48
Nodes (6): fixture(), optional_null_is_accepted_then_omitted(), probe_roundtrips_without_losing_precision(), rejects_invalid_probe_boundaries(), Value, scenario_and_version_are_validated()

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

### Community 86 - "Community 86"
Cohesion: 0.50
Nodes (3): rejects_invalid_request_semantics(), request(), Value

### Community 87 - "Community 87"
Cohesion: 0.70
Nodes (4): java_environment(), main(), Explicit local SDK setup and demo build; never runs checks or accepts SDK…, setup()

### Community 88 - "Community 88"
Cohesion: 0.50
Nodes (3): Model, Relation, Uuid

### Community 89 - "Community 89"
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
- **149 isolated node(s):** `ErrorNoticeProps`, `PageHeadingProps`, `MAX_APK`, `SESSION_SECONDS`, `UPLOAD_SECONDS` (+144 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 666 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **28 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `DiscoveryReply` connect `Community 18` to `Community 14`, `Community 30`?**
  _High betweenness centrality (0.144) - this node is a cross-community bridge._
- **Why does `QualificationError` connect `Community 14` to `Community 5`, `Community 6`, `Community 37`, `Community 11`, `Community 12`, `Community 13`, `Community 16`, `Community 24`?**
  _High betweenness centrality (0.130) - this node is a cross-community bridge._
- **Why does `explore()` connect `Community 14` to `Community 13`, `Community 18`, `Community 12`, `Community 5`?**
  _High betweenness centrality (0.087) - this node is a cross-community bridge._
- **Are the 45 inferred relationships involving `QualificationError` (e.g. with `DiscoveryBroker` and `BrokerClient`) actually correct?**
  _`QualificationError` has 45 INFERRED edges - model-reasoned connections that need verification._
- **Are the 49 inferred relationships involving `field()` (e.g. with `build()` and `cleanup_pending()`) actually correct?**
  _`field()` has 49 INFERRED edges - model-reasoned connections that need verification._
- **Are the 47 inferred relationships involving `one()` (e.g. with `artifact()` and `build()`) actually correct?**
  _`one()` has 47 INFERRED edges - model-reasoned connections that need verification._
- **What connects `ErrorNoticeProps`, `PageHeadingProps`, `MAX_APK` to the rest of the system?**
  _149 weakly-connected nodes found - possible documentation gaps or missing edges._