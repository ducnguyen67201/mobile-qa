# Graph Report - mobile-qa  (2026-09-18)

## Corpus Check
- cluster-only mode — file stats not available

## Summary
- 2298 nodes · 6416 edges · 136 communities (91 shown, 29 thin omitted)
- Extraction: 93% EXTRACTED · 7% INFERRED · 0% AMBIGUOUS · INFERRED: 464 edges (avg confidence: 0.88)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `edd561cf`
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
- Community 89
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
- Community 109
- Community 110
- Community 111
- Community 112
- Community 113
- Community 115
- Community 116
- Community 117
- Community 118
- Community 119
- Community 120
- Community 135

## God Nodes (most connected - your core abstractions)
1. `QualificationError` - 161 edges
2. `field()` - 55 edges
3. `one()` - 54 edges
4. `Profile` - 52 edges
5. `useWorkspace()` - 52 edges
6. `checked()` - 50 edges
7. `@mantine/core` - 46 edges
8. `Session` - 42 edges
9. `exec()` - 38 edges
10. `rows()` - 38 edges

## Surprising Connections (you probably didn't know these)
- `create()` --indirect_call--> `profile()`  [INFERRED]
  scripts/test_library_smoke.py → apps/mobile-worker/tests/test_device.py
- `task()` --indirect_call--> `client()`  [INFERRED]
  apps/mobile-worker/src/mobile_qa_worker/authoring/minitap_discovery.py → scripts/app_setup_smoke.py
- `logout()` --references--> `LogoutResponse`  [EXTRACTED]
  apps/api/src/controllers/setup.rs → crates/contracts/src/browser.rs
- `test_usage_deduplicates_and_keeps_unknown()` --uses--> `UsageRecorder`  [INFERRED]
  apps/mobile-worker/tests/test_sdk_adapter.py → apps/mobile-worker/src/mobile_qa_worker/qualification/sdk_adapter.py
- `test_checkpoint_deadline_respects_approved_window()` --uses--> `QualificationError`  [INFERRED]
  apps/mobile-worker/tests/test_execution.py → apps/mobile-worker/src/mobile_qa_worker/qualification/config.py

## Import Cycles
- None detected.

## Communities (136 total, 29 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.05
Nodes (147): acknowledge(), recorded(), require(), ApiResult, AppContext, ConnectionTrait, Uuid, conflict() (+139 more)

### Community 1 - "Community 1"
Cohesion: 0.05
Nodes (107): build(), BuildQuery, claim(), cleanup(), complete(), events(), heartbeat(), legacy_receipt() (+99 more)

### Community 2 - "Community 2"
Cohesion: 0.06
Nodes (103): artifact(), cancel(), create(), detail(), list(), ListQuery, preview(), ApiResult (+95 more)

### Community 3 - "Community 3"
Cohesion: 0.07
Nodes (65): every_declared_route_requires_the_declared_security_and_errors(), google_identity_binding_expiry_replay_and_password_removal(), persisted_workflow_auth_scope_and_real_validation(), public_registration_approval_and_multiple_workspace_isolation(), rejection_recovery_and_infrastructure_failure(), throttles_quota_and_revision_scope(), approved(), clean_start_route_is_fenced_and_recovery_keeps_original_evidence() (+57 more)

### Community 4 - "Community 4"
Cohesion: 0.07
Nodes (53): b64(), client(), google_fixture(), google_sign_in(), main(), provision(), Secret-free HTTP acceptance using owned test accounts/processes and a synthetic…, Ephemeral RSA fixture keys; the API accepts these only in Environment::Test. (+45 more)

### Community 5 - "Community 5"
Cohesion: 0.07
Nodes (45): explore(), Path, Bounded pipe protocol; SDK logging goes to /dev/null, never into public…, invoke(), Path, Bounded discovery drives the same direct commands testers record and rerun., run(), ask() (+37 more)

### Community 6 - "Community 6"
Cohesion: 0.07
Nodes (47): BrokerClient, Path, Pinned SDK graph adaptation. All device/model observations cross the parent…, run(), Path, run(), compact(), main() (+39 more)

### Community 7 - "Community 7"
Cohesion: 0.12
Nodes (50): checked(), forgetSession(), headers(), options, setCsrfToken(), buildsQuery(), completeUpload(), createApp() (+42 more)

### Community 8 - "Community 8"
Cohesion: 0.08
Nodes (40): ApiError, ApkMetadata, AppListResponse, AppResponse, ApprovalStatus, AppSummary, BrowserApi, BuildListResponse (+32 more)

### Community 9 - "Community 9"
Cohesion: 0.09
Nodes (33): DiscoveryBroker, Parent-owned discovery effects. The SDK receives observations, never device…, Use the same pixel mask for retained evidence and provider requests., redact(), check(), execute(), hierarchy(), nodes() (+25 more)

### Community 10 - "Community 10"
Cohesion: 0.06
Nodes (32): Evidence, Path, safe_path(), validate_result(), One-device operator qualification; no production scheduler or customer…, test_campaign_oracle_never_reaches_runner(), run(), test_campaign_schedule_and_accounting() (+24 more)

### Community 11 - "Community 11"
Cohesion: 0.08
Nodes (45): configure_auth(), MAX_APK, AppContext, Config, Environment, Option, PathBuf, Result (+37 more)

### Community 12 - "Community 12"
Cohesion: 0.06
Nodes (39): host_environment(), Owned Android operations, independent of app-specific qualification oracles., device_for(), execute(), png(), Path, Synthetic HTTP-worker evidence. It is never advertised as device qualification., Durable HTTP execution client; separate from the qualification experiment. (+31 more)

### Community 13 - "Community 13"
Cohesion: 0.11
Nodes (31): profile, second, routes, apiError(), app, appId, build, buildId (+23 more)

### Community 14 - "Community 14"
Cohesion: 0.05
Nodes (35): Any, Explicit bounded AI authoring; ordinary device operations never enter this…, check_context(), compact(), configure_agent(), __call__(), direct_tool(), foreground() (+27 more)

### Community 15 - "Community 15"
Cohesion: 0.10
Nodes (39): Own one disposable AVD. Refuse existing emulator-5554 rather than borrowing it., execute(), Path, Sequential approved actions on the qualified demo adapter, with supervised SDK…, sdk_action(), Explicit app policies. Generic direct execution never uses the demo's backend…, await_start_ack(), execute() (+31 more)

### Community 16 - "Community 16"
Cohesion: 0.10
Nodes (38): runsQuery(), appQuery(), appsQuery(), App(), AccountMenu(), Navigation(), NavigationItem(), navigationLinks (+30 more)

### Community 17 - "Community 17"
Cohesion: 0.13
Nodes (28): buildQuery(), settingsQuery, ApkUpload(), AppForm(), parseOrigins(), AttemptReadiness(), BuildDetail(), colors (+20 more)

### Community 18 - "Community 18"
Cohesion: 0.12
Nodes (31): openPhone(), phoneOptionsQuery(), phoneQuery(), stopPhone(), cancelTestGeneration(), generateTests(), runPhoneCommand(), saveAuthoredTests() (+23 more)

### Community 19 - "Community 19"
Cohesion: 0.11
Nodes (28): ApiFailure, DbErr, Error, From, Into, Option, Response, Self (+20 more)

### Community 20 - "Community 20"
Cohesion: 0.17
Nodes (34): AuthoringModelRequest, AuthoringModelResponse, AuthoringUsage, AutomationSequence, CoverageKind, DirectCommand, DiscoveryCall, DiscoveryDecision (+26 more)

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
Cohesion: 0.17
Nodes (16): ArtifactStore, private_immutable_publication_and_symlink_rejection(), ApiResult, DateTime, File, Option, Path, PathBuf (+8 more)

### Community 25 - "Community 25"
Cohesion: 0.12
Nodes (21): Device, parametrize, Deterministic device seam; no emulator, network or model credentials., Rpc, test_cancel_before_action_has_no_effect(), test_capture_shares_active_rpc_instead_of_starting_a_second_instrumentation(), test_literal_set_text_uses_rpc_without_model(), test_rejects_ambiguous_disabled_secret_outside_targets() (+13 more)

### Community 26 - "Community 26"
Cohesion: 0.14
Nodes (18): Device, Path, Controlled demo qualification policy layered on owned Android operations., Observation, observe(), Deterministic oracle for the controlled persistence demo, not arbitrary…, validate_png(), verdict() (+10 more)

### Community 27 - "Community 27"
Cohesion: 0.32
Nodes (20): authorized(), create(), detail(), environment(), list(), ListQuery, membership(), origins() (+12 more)

### Community 28 - "Community 28"
Cohesion: 0.10
Nodes (19): name, packageManager, private, type, version, eslint, @eslint/js, globals (+11 more)

### Community 29 - "Community 29"
Cohesion: 0.10
Nodes (20): compilerOptions, allowImportingTsExtensions, baseUrl, esModuleInterop, jsx, lib, module, moduleResolution (+12 more)

### Community 30 - "Community 30"
Cohesion: 0.16
Nodes (18): ContractProbe, FakeExecutionRequest, FakeExecutionResult, nullable_string_schema(), Outcome, required_nullable(), DateTime, Error (+10 more)

### Community 31 - "Community 31"
Cohesion: 0.37
Nodes (17): build(), claim(), detail(), open(), options(), ApiResult, AppContext, HeaderMap (+9 more)

### Community 32 - "Community 32"
Cohesion: 0.29
Nodes (18): OpenPhoneRequest, PhoneBuildChoice, PhoneClaimRequest, PhoneClaimResponse, PhoneControl, PhoneFrame, PhoneLease, PhoneOptions (+10 more)

### Community 33 - "Community 33"
Cohesion: 0.16
Nodes (13): execute(), parse_request(), Return the result chosen by a fixture; do not discover whether an app works.…, Validate fixture JSON without coercing values such as string versions to…, Echo the run ID and map the requested scenario to its fixed simulated outcome.…, parametrize, Python half of the shared fixture contract and local CLI checks. No device SDK…, test_cli_invalid_nonzero() (+5 more)

### Community 34 - "Community 34"
Cohesion: 0.11
Nodes (18): devDependencies, eslint, @eslint/js, globals, @hey-api/openapi-ts, jsdom, prettier, @testing-library/jest-dom (+10 more)

### Community 35 - "Community 35"
Cohesion: 0.24
Nodes (14): arg(), execute(), Execution, id(), purpose(), read(), ApiResult, AppContext (+6 more)

### Community 37 - "Community 37"
Cohesion: 0.42
Nodes (15): build(), build_response(), complete(), create(), list(), ApiResult, AppContext, ListQuery (+7 more)

### Community 38 - "Community 38"
Cohesion: 0.23
Nodes (13): arg(), execute(), id(), Operator, ApiResult, AppContext, Result, String (+5 more)

### Community 40 - "Community 40"
Cohesion: 0.48
Nodes (12): cancel(), command(), detail(), generate(), ApiResult, AppContext, Json, Path (+4 more)

### Community 41 - "Community 41"
Cohesion: 0.32
Nodes (7): cancelRun(), createRun(), planQuery(), runQuery(), RunPreview(), actionLabel(), RunDetail()

### Community 42 - "Community 42"
Cohesion: 0.32
Nodes (6): android.app.Activity, android.os.Bundle, android.widget.LinearLayout, MainActivity, LinearLayout, Override

### Community 43 - "Community 43"
Cohesion: 0.17
Nodes (4): parametrize, test_full_supervisor_with_injected_device(), boot(), sdk()

### Community 44 - "Community 44"
Cohesion: 0.26
Nodes (6): healthQuery, apiClient, ApiClientError, requestMethods, transportClient, healthy

### Community 45 - "Community 45"
Cohesion: 0.17
Nodes (11): binaryUpload, completeStatuses, editableDefinition, healthy, implicitDefault, persistedBuild, staleBuild, staleResponse (+3 more)

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

### Community 51 - "Community 51"
Cohesion: 0.31
Nodes (7): current(), method_not_allowed(), Json, StatusCode, unknown(), HealthResponse, HealthStatus

### Community 52 - "Community 52"
Cohesion: 0.25
Nodes (3): DeviceRpc, ElementRpc, Protocol

### Community 53 - "Community 53"
Cohesion: 0.25
Nodes (6): parametrize, Synthetic screen context only; no emulator, model or network access., test_controls_filter_passwords_and_preserve_bounds(), test_invalid_or_outside_controls(), test_session_binds_image_and_model_before_side_effects(), test_shutdown_during_empty_claim_leaves_no_dirty_marker()

### Community 54 - "Community 54"
Cohesion: 0.22
Nodes (9): scripts, build, dev, format, format:check, generate:sdk, lint, test (+1 more)

### Community 55 - "Community 55"
Cohesion: 0.42
Nodes (7): BudgetFields(), moveItem(), Ordering(), CaseMembership(), choiceLabel(), PlanFields(), SuiteFields()

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
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 63 - "Community 63"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 64 - "Community 64"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 65 - "Community 65"
Cohesion: 0.25
Nodes (7): Model, Relation, DateTimeUtc, Json, Option, String, Uuid

### Community 66 - "Community 66"
Cohesion: 0.25
Nodes (7): Model, Relation, DateTimeUtc, Json, Option, String, Uuid

### Community 67 - "Community 67"
Cohesion: 0.36
Nodes (5): installer(), parametrize, Exercise the pinned installer with local archives only; never contact Google., test_install_checks_archive_and_preserves_managed_metadata(), test_installer_only_selects_native_system_image()

### Community 68 - "Community 68"
Cohesion: 0.54
Nodes (5): startGoogleSignIn(), useGoogleSignIn(), safeReturnTo(), SignIn(), @react-oauth/google

### Community 70 - "Community 70"
Cohesion: 0.39
Nodes (5): approved_import_shape_has_separate_actions_and_checks(), case(), invalid_definition_references_and_paths_are_rejected(), manual_methods_are_defined_but_not_implicitly_executable(), unknown_fields_are_not_imported()

### Community 71 - "Community 71"
Cohesion: 0.29
Nodes (5): Migrator, Box, MigrationTrait, Vec, MigratorTrait

### Community 72 - "Community 72"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 73 - "Community 73"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 74 - "Community 74"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 75 - "Community 75"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 76 - "Community 76"
Cohesion: 0.43
Nodes (6): discovery_progress(), ApiResult, Option, T, same(), task_update()

### Community 77 - "Community 77"
Cohesion: 0.38
Nodes (3): target(), DirectTarget, Result

### Community 78 - "Community 78"
Cohesion: 0.48
Nodes (6): fixture(), optional_null_is_accepted_then_omitted(), probe_roundtrips_without_losing_precision(), rejects_invalid_probe_boundaries(), Value, scenario_and_version_are_validated()

### Community 79 - "Community 79"
Cohesion: 0.43
Nodes (4): case(), drafts_round_trip_and_publication_preserves_the_execution_shape(), incomplete_case_is_saveable_but_not_publishable(), oversized_unknown_and_forged_shapes_are_rejected()

### Community 80 - "Community 80"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 81 - "Community 81"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 82 - "Community 82"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 83 - "Community 83"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 84 - "Community 84"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 85 - "Community 85"
Cohesion: 0.47
Nodes (5): create(), ApiResult, AppContext, Uuid, CreateWorkspaceRequest

### Community 86 - "Community 86"
Cohesion: 0.40
Nodes (4): request_context(), Response, Next, Request

### Community 87 - "Community 87"
Cohesion: 0.40
Nodes (4): Model, Relation, String, Uuid

### Community 88 - "Community 88"
Cohesion: 0.50
Nodes (3): Protocol, StructuredCall, StructuredModel

### Community 89 - "Community 89"
Cohesion: 0.40
Nodes (4): main(), Box, Error, Result

### Community 91 - "Community 91"
Cohesion: 0.50
Nodes (3): rejects_invalid_request_semantics(), request(), Value

### Community 92 - "Community 92"
Cohesion: 0.70
Nodes (4): java_environment(), main(), Explicit local SDK setup and demo build; never runs checks or accepts SDK…, setup()

### Community 93 - "Community 93"
Cohesion: 0.50
Nodes (3): Model, Relation, Uuid

### Community 94 - "Community 94"
Cohesion: 0.83
Nodes (3): gradlew script, die(), warn()

### Community 96 - "Community 96"
Cohesion: 0.67
Nodes (3): main(), Explicit repository formatting; generators retain ownership of generated output., run()

### Community 97 - "Community 97"
Cohesion: 0.67
Nodes (3): install(), main(), Explicit project-local Android intake tool installation, never run at API…

### Community 99 - "Community 99"
Cohesion: 0.67
Nodes (3): migration, mobile-qa, mobile-qa-contracts

## Knowledge Gaps
- **149 isolated node(s):** `ErrorNoticeProps`, `PageHeadingProps`, `ActiveModel`, `ActiveModel`, `ActiveModel` (+144 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 684 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **29 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `QualificationError` connect `Community 9` to `Community 5`, `Community 6`, `Community 10`, `Community 43`, `Community 12`, `Community 14`, `Community 15`, `Community 53`, `Community 25`, `Community 26`?**
  _High betweenness centrality (0.158) - this node is a cross-community bridge._
- **Why does `DiscoveryReply` connect `Community 20` to `Community 9`, `Community 5`, `Community 30`?**
  _High betweenness centrality (0.151) - this node is a cross-community bridge._
- **Why does `Session` connect `Community 2` to `Community 40`, `Community 19`, `Community 37`, `Community 31`?**
  _High betweenness centrality (0.111) - this node is a cross-community bridge._
- **Are the 53 inferred relationships involving `QualificationError` (e.g. with `DiscoveryBroker` and `BrokerClient`) actually correct?**
  _`QualificationError` has 53 INFERRED edges - model-reasoned connections that need verification._
- **Are the 51 inferred relationships involving `field()` (e.g. with `build()` and `acknowledge()`) actually correct?**
  _`field()` has 51 INFERRED edges - model-reasoned connections that need verification._
- **Are the 48 inferred relationships involving `one()` (e.g. with `artifact()` and `build()`) actually correct?**
  _`one()` has 48 INFERRED edges - model-reasoned connections that need verification._
- **Are the 30 inferred relationships involving `Profile` (e.g. with `explore()` and `invoke()`) actually correct?**
  _`Profile` has 30 INFERRED edges - model-reasoned connections that need verification._