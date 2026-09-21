# Graph Report - mobile-qa  (2026-09-21)

## Corpus Check
- cluster-only mode — file stats not available

## Summary
- 2561 nodes · 7159 edges · 151 communities (105 shown, 29 thin omitted)
- Extraction: 93% EXTRACTED · 7% INFERRED · 0% AMBIGUOUS · INFERRED: 514 edges (avg confidence: 0.88)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `c4550b93`
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
- Community 114
- Community 115
- Community 116
- Community 117
- Community 118
- Community 119
- Community 120
- Community 121
- Community 122
- Community 123
- Community 124
- Community 125
- Community 126
- Community 128
- Community 129
- Community 130
- Community 131
- Community 132
- Community 133
- Community 134
- Community 150

## God Nodes (most connected - your core abstractions)
1. `QualificationError` - 171 edges
2. `field()` - 61 edges
3. `Profile` - 58 edges
4. `useWorkspace()` - 58 edges
5. `@mantine/core` - 57 edges
6. `one()` - 56 edges
7. `checked()` - 53 edges
8. `exec()` - 46 edges
9. `@tanstack/react-query` - 44 edges
10. `rows()` - 44 edges

## Surprising Connections (you probably didn't know these)
- `qualify()` --uses--> `AndroidDevice`  [INFERRED]
  scripts/regression_acceptance.py → apps/mobile-worker/src/mobile_qa_worker/device/android.py
- `create()` --indirect_call--> `profile()`  [INFERRED]
  scripts/test_library_smoke.py → apps/mobile-worker/tests/test_device.py
- `qualify()` --calls--> `check()`  [INFERRED]
  scripts/regression_acceptance.py → apps/mobile-worker/src/mobile_qa_worker/automation/direct.py
- `task()` --indirect_call--> `client()`  [INFERRED]
  apps/mobile-worker/src/mobile_qa_worker/authoring/minitap_discovery.py → scripts/app_setup_smoke.py
- `logout()` --references--> `LogoutResponse`  [EXTRACTED]
  apps/api/src/controllers/setup.rs → crates/contracts/src/browser.rs

## Import Cycles
- None detected.

## Communities (151 total, 29 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.05
Nodes (92): cancel(), command(), detail(), generate(), ApiResult, AppContext, Json, Path (+84 more)

### Community 1 - "Community 1"
Cohesion: 0.08
Nodes (74): archive(), create(), default_plan(), draft(), entry(), list(), options(), ApiResult (+66 more)

### Community 2 - "Community 2"
Cohesion: 0.08
Nodes (63): doctor(), host_environment(), execute(), Path, Sequential approved actions on the qualified demo adapter, with supervised SDK…, sdk_action(), Explicit app policies. Generic direct execution never uses the demo's backend…, await_start_ack() (+55 more)

### Community 3 - "Community 3"
Cohesion: 0.07
Nodes (56): DiscoveryBroker, explore(), Path, Parent-owned discovery effects. The SDK receives observations, never device…, Bounded pipe protocol; SDK logging goes to /dev/null, never into public…, invoke(), Path, Bounded discovery drives the same direct commands testers record and rerun. (+48 more)

### Community 4 - "Community 4"
Cohesion: 0.04
Nodes (59): BrokerClient, check_context(), compact(), configure_agent(), bounded_model(), __call__(), direct_tool(), foreground() (+51 more)

### Community 5 - "Community 5"
Cohesion: 0.06
Nodes (45): mocks, mocks, session, issue, LibraryCoverageFlow(), attempt(), authentication, backendUnavailable (+37 more)

### Community 6 - "Community 6"
Cohesion: 0.06
Nodes (68): every_declared_route_requires_the_declared_security_and_errors(), google_identity_binding_expiry_replay_and_password_removal(), persisted_workflow_auth_scope_and_real_validation(), public_registration_approval_and_multiple_workspace_isolation(), rejection_recovery_and_infrastructure_failure(), throttles_quota_and_revision_scope(), assertion_absence_requires_a_ready_unambiguous_screen(), clean_start_route_is_fenced_and_recovery_keeps_original_evidence() (+60 more)

### Community 7 - "Community 7"
Cohesion: 0.07
Nodes (59): b64(), client(), google_fixture(), google_sign_in(), main(), provision(), Secret-free HTTP acceptance using owned test accounts/processes and a synthetic…, Ephemeral RSA fixture keys; the API accepts these only in Environment::Test. (+51 more)

### Community 8 - "Community 8"
Cohesion: 0.10
Nodes (64): artifact(), cancel(), case_create(), case_preview(), create(), detail(), history(), HistoryQuery (+56 more)

### Community 9 - "Community 9"
Cohesion: 0.04
Nodes (44): Observation, observe(), Deterministic oracle for the controlled persistence demo, not arbitrary…, verdict(), parametrize, test_campaign_oracle_never_reaches_runner(), run(), test_campaign_schedule_and_accounting() (+36 more)

### Community 10 - "Community 10"
Cohesion: 0.08
Nodes (40): ApiError, ApkMetadata, AppListResponse, AppResponse, ApprovalStatus, AppSummary, BrowserApi, BuildListResponse (+32 more)

### Community 11 - "Community 11"
Cohesion: 0.06
Nodes (45): Owned Android operations, independent of app-specific qualification oracles., device_for(), execute(), png(), Path, Synthetic HTTP-worker evidence. It is never advertised as device qualification., Durable HTTP execution client; separate from the qualification experiment., assignment() (+37 more)

### Community 12 - "Community 12"
Cohesion: 0.12
Nodes (53): String, summary(), ActionKind, ApprovalPurpose, ArtifactReceipt, ArtifactRequest, AttemptResponse, CaseDefinition (+45 more)

### Community 13 - "Community 13"
Cohesion: 0.12
Nodes (39): appsQuery(), createWorkspace(), sessionQuery, signOut(), App(), AccountMenu(), ErrorNotice(), ErrorNoticeProps (+31 more)

### Community 14 - "Community 14"
Cohesion: 0.08
Nodes (45): configure_auth(), MAX_APK, AppContext, Config, Environment, Option, PathBuf, Result (+37 more)

### Community 15 - "Community 15"
Cohesion: 0.08
Nodes (34): Client, NoRedirect, Exception, T, Bounded, same-origin HTTP; generated Pydantic validates every JSON response., TransportError, Path, No token is persisted. An active local attempt requires recovery, never blind… (+26 more)

### Community 16 - "Community 16"
Cohesion: 0.11
Nodes (38): createApp(), saveEnvironment(), createLibraryEntry(), defaultPlanQuery(), libraryDraftQuery(), libraryEntryQuery(), libraryErrorDetails(), libraryHistoryQuery() (+30 more)

### Community 17 - "Community 17"
Cohesion: 0.12
Nodes (33): cancelTestGeneration(), generateTests(), runPhoneCommand(), saveAuthoredTests(), templatesQuery(), PhoneWorkspace(), ResizableWorkspace(), commandTarget() (+25 more)

### Community 18 - "Community 18"
Cohesion: 0.09
Nodes (33): definition(), registry_is_immutable_resolvable_and_retirable(), BTreeMap, ModelBinding, ModelCapability, ModelDefinition, ModelProvider, ModelReference (+25 more)

### Community 19 - "Community 19"
Cohesion: 0.11
Nodes (28): appQuery(), buildQuery(), buildsQuery(), completeUpload(), createUpload(), getUpload(), settingsQuery, signIn() (+20 more)

### Community 20 - "Community 20"
Cohesion: 0.11
Nodes (28): ApiFailure, DbErr, Error, From, Into, Option, Response, Self (+20 more)

### Community 21 - "Community 21"
Cohesion: 0.13
Nodes (34): create(), manifest(), preview(), ApiResult, AppContext, ConnectionTrait, Uuid, create() (+26 more)

### Community 22 - "Community 22"
Cohesion: 0.10
Nodes (28): attemptStatus(), buildLibraryCoverageFlow(), buildRunCoverageFlow(), caseCount(), caseVersion(), choiceLabel(), CoverageCanvasEditor(), CoverageFlowModel (+20 more)

### Community 23 - "Community 23"
Cohesion: 0.20
Nodes (23): caseRunPreviewQuery(), createCaseRun(), createSuiteRun(), historyQuery(), suiteRunPreviewQuery(), createRun(), planQuery(), runsQuery() (+15 more)

### Community 24 - "Community 24"
Cohesion: 0.10
Nodes (17): AppRoutes, App, AppContext, Box, Config, Environment, Path, Result (+9 more)

### Community 25 - "Community 25"
Cohesion: 0.29
Nodes (28): complete_upload(), create_app(), create_upload(), create_workspace(), get_app(), get_build(), get_upload(), google_challenge() (+20 more)

### Community 26 - "Community 26"
Cohesion: 0.17
Nodes (16): ArtifactStore, private_immutable_publication_and_symlink_rejection(), ApiResult, DateTime, File, Option, Path, PathBuf (+8 more)

### Community 27 - "Community 27"
Cohesion: 0.29
Nodes (24): build(), BuildQuery, claim(), cleanup(), complete(), events(), heartbeat(), legacy_receipt() (+16 more)

### Community 28 - "Community 28"
Cohesion: 0.11
Nodes (6): One-device operator qualification; no production scheduler or customer…, UsageRecorder, model(), test_qualification_rejects_non_navigation_model_before_provider_setup(), test_sdk_exact_public_seam(), test_usage_deduplicates_and_keeps_unknown()

### Community 29 - "Community 29"
Cohesion: 0.16
Nodes (14): cancelRun(), runQuery(), labels, observed(), RunResult(), SessionResult(), item, actionLabel() (+6 more)

### Community 30 - "Community 30"
Cohesion: 0.27
Nodes (21): conflict(), claim(), claim_wait(), cleanup(), complete(), events(), heartbeat(), lease() (+13 more)

### Community 31 - "Community 31"
Cohesion: 0.10
Nodes (20): name, packageManager, private, type, version, eslint, @eslint/js, globals (+12 more)

### Community 32 - "Community 32"
Cohesion: 0.32
Nodes (20): authorized(), create(), detail(), environment(), list(), ListQuery, membership(), origins() (+12 more)

### Community 33 - "Community 33"
Cohesion: 0.16
Nodes (12): adapter(), assigned(), case_matches(), duration(), ApiResult, ConnectionTrait, String, Uuid (+4 more)

### Community 34 - "Community 34"
Cohesion: 0.15
Nodes (18): Explicit bounded AI authoring; ordinary device operations never enter this…, call(), parametrize, Discovery guards with synthetic device effects; the separate SDK fixture uses…, seam(), task(), test_budget_reserves_one_drafting_call_and_counts_unfinished_calls(), test_changed_foreground_and_oversized_context_fail_closed() (+10 more)

### Community 35 - "Community 35"
Cohesion: 0.10
Nodes (20): compilerOptions, allowImportingTsExtensions, baseUrl, esModuleInterop, jsx, lib, module, moduleResolution (+12 more)

### Community 36 - "Community 36"
Cohesion: 0.32
Nodes (19): one(), rows(), ConnectionTrait, QueryResult, Vec, assemble(), attempt(), authorize() (+11 more)

### Community 37 - "Community 37"
Cohesion: 0.42
Nodes (19): authorized(), claim(), detail(), lease(), open(), options(), read(), reconcile() (+11 more)

### Community 38 - "Community 38"
Cohesion: 0.09
Nodes (11): binaryUpload, completeStatuses, editableDefinition, healthy, implicitDefault, persistedBuild, staleBuild, staleResponse (+3 more)

### Community 39 - "Community 39"
Cohesion: 0.17
Nodes (11): Device, parametrize, Deterministic device seam; no emulator, network or model credentials., Rpc, test_cancel_before_action_has_no_effect(), test_capture_shares_active_rpc_instead_of_starting_a_second_instrumentation(), test_literal_set_text_uses_rpc_without_model(), test_rejects_ambiguous_disabled_secret_outside_targets() (+3 more)

### Community 40 - "Community 40"
Cohesion: 0.18
Nodes (16): decode(), exec(), hash(), json(), ApiResult, String, T, Value (+8 more)

### Community 41 - "Community 41"
Cohesion: 0.16
Nodes (13): execute(), parse_request(), Return the result chosen by a fixture; do not discover whether an app works.…, Validate fixture JSON without coercing values such as string versions to…, Echo the run ID and map the requested scenario to its fixed simulated outcome.…, parametrize, Python half of the shared fixture contract and local CLI checks. No device SDK…, test_cli_invalid_nonzero() (+5 more)

### Community 42 - "Community 42"
Cohesion: 0.11
Nodes (18): devDependencies, eslint, @eslint/js, globals, @hey-api/openapi-ts, jsdom, prettier, @testing-library/jest-dom (+10 more)

### Community 43 - "Community 43"
Cohesion: 0.15
Nodes (13): list(), ApiResult, AppContext, Option, String, Uuid, register(), ApiResult (+5 more)

### Community 44 - "Community 44"
Cohesion: 0.28
Nodes (14): field(), cleanup_pending(), content(), record(), reserve(), ApiResult, AppContext, Body (+6 more)

### Community 45 - "Community 45"
Cohesion: 0.42
Nodes (15): build(), build_response(), complete(), create(), list(), ApiResult, AppContext, ListQuery (+7 more)

### Community 46 - "Community 46"
Cohesion: 0.23
Nodes (13): arg(), execute(), Execution, id(), read(), ApiResult, AppContext, Result (+5 more)

### Community 47 - "Community 47"
Cohesion: 0.23
Nodes (13): arg(), execute(), id(), Operator, ApiResult, AppContext, Result, String (+5 more)

### Community 48 - "Community 48"
Cohesion: 0.30
Nodes (13): compare(), compares_verified_facts_and_freezes_baseline_identity(), context_matches(), coverage_changes_remain_visible(), eligible(), failed(), manifest_legacy_serialization_is_unchanged(), outcome() (+5 more)

### Community 49 - "Community 49"
Cohesion: 0.36
Nodes (14): action(), cancel(), command(), enqueue(), generate(), job_detail(), ApiResult, AppContext (+6 more)

### Community 50 - "Community 50"
Cohesion: 0.25
Nodes (12): ExecutionContextV1, PreflightAcknowledgement, PreflightReceipt, PreflightRequest, RecoveryEvent, DateTime, Result, String (+4 more)

### Community 51 - "Community 51"
Cohesion: 0.25
Nodes (11): BudgetFields(), moveItem(), Ordering(), CaseMembership(), choiceLabel(), countLabel(), PlanFields(), SuiteFields() (+3 more)

### Community 53 - "Community 53"
Cohesion: 0.38
Nodes (12): advertise(), register(), resolve(), resolve_for_new_work(), resolve_legacy(), retire(), ApiResult, ConnectionTrait (+4 more)

### Community 54 - "Community 54"
Cohesion: 0.50
Nodes (12): get(), import(), operator(), profile(), register_profile(), require_case(), resolve(), ApiResult (+4 more)

### Community 55 - "Community 55"
Cohesion: 0.37
Nodes (10): apply(), bump(), link_import(), Mutation, ApiResult, AppContext, ConnectionTrait, Option (+2 more)

### Community 56 - "Community 56"
Cohesion: 0.32
Nodes (6): android.app.Activity, android.os.Bundle, android.widget.LinearLayout, MainActivity, LinearLayout, Override

### Community 57 - "Community 57"
Cohesion: 0.20
Nodes (5): Broker, main(), Offline actual Agent.run_task graph with scripted LangChain model responses,…, ScriptedModel, BaseChatModel

### Community 58 - "Community 58"
Cohesion: 0.17
Nodes (12): dependencies, lucide-react, @mantine/core, @mantine/form, @mantine/hooks, react, react-dom, @react-oauth/google (+4 more)

### Community 59 - "Community 59"
Cohesion: 0.26
Nodes (6): healthQuery, apiClient, ApiClientError, requestMethods, transportClient, healthy

### Community 60 - "Community 60"
Cohesion: 0.26
Nodes (9): files(), main(), Path, Generate both consumer contracts from Rust without starting the application.…, Read managed output bytes keyed by repo-relative path, independent of mtimes., Compare the whole generated set, including new and removed files. Preserve…, synchronize(), ExportTests (+1 more)

### Community 61 - "Community 61"
Cohesion: 0.27
Nodes (10): evaluate(), observe(), png_valid(), ApiResult, AppContext, Outcome, Result, String (+2 more)

### Community 62 - "Community 62"
Cohesion: 0.22
Nodes (8): Cleanup, execute(), ApiResult, AppContext, Result, Task, TaskInfo, Vars

### Community 63 - "Community 63"
Cohesion: 0.24
Nodes (8): ExecutionWakeup, notify(), AppContext, Self, subscribe(), Default, Receiver, Sender

### Community 65 - "Community 65"
Cohesion: 0.31
Nodes (7): current(), method_not_allowed(), Json, StatusCode, unknown(), HealthResponse, HealthStatus

### Community 66 - "Community 66"
Cohesion: 0.25
Nodes (3): DeviceRpc, ElementRpc, Protocol

### Community 67 - "Community 67"
Cohesion: 0.22
Nodes (9): scripts, build, dev, format, format:check, generate:sdk, lint, test (+1 more)

### Community 69 - "Community 69"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 70 - "Community 70"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 71 - "Community 71"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 72 - "Community 72"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 73 - "Community 73"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 74 - "Community 74"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 75 - "Community 75"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 76 - "Community 76"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 77 - "Community 77"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 78 - "Community 78"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 79 - "Community 79"
Cohesion: 0.25
Nodes (7): Model, Relation, DateTimeUtc, Json, Option, String, Uuid

### Community 80 - "Community 80"
Cohesion: 0.25
Nodes (7): Model, Relation, DateTimeUtc, Json, Option, String, Uuid

### Community 81 - "Community 81"
Cohesion: 0.54
Nodes (7): acknowledge(), recorded(), require(), ApiResult, AppContext, ConnectionTrait, Uuid

### Community 82 - "Community 82"
Cohesion: 0.36
Nodes (5): installer(), parametrize, Exercise the pinned installer with local archives only; never contact Google., test_install_checks_archive_and_preserves_managed_metadata(), test_installer_only_selects_native_system_image()

### Community 85 - "Community 85"
Cohesion: 0.39
Nodes (5): approved_import_shape_has_separate_actions_and_checks(), case(), invalid_definition_references_and_paths_are_rejected(), manual_methods_are_defined_but_not_implicitly_executable(), unknown_fields_are_not_imported()

### Community 86 - "Community 86"
Cohesion: 0.39
Nodes (5): case(), drafts_round_trip_and_publication_preserves_the_execution_shape(), incomplete_case_is_saveable_but_not_publishable(), oversized_unknown_and_forged_shapes_are_rejected(), snapshot_normalization_preserves_ai_editor_provenance()

### Community 87 - "Community 87"
Cohesion: 0.29
Nodes (5): Migrator, Box, MigrationTrait, Vec, MigratorTrait

### Community 88 - "Community 88"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 89 - "Community 89"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 90 - "Community 90"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 91 - "Community 91"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 92 - "Community 92"
Cohesion: 0.48
Nodes (6): fixture(), optional_null_is_accepted_then_omitted(), probe_roundtrips_without_losing_precision(), rejects_invalid_probe_boundaries(), Value, scenario_and_version_are_validated()

### Community 93 - "Community 93"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 94 - "Community 94"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 95 - "Community 95"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 96 - "Community 96"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 97 - "Community 97"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 98 - "Community 98"
Cohesion: 0.47
Nodes (5): create(), ApiResult, AppContext, Uuid, CreateWorkspaceRequest

### Community 99 - "Community 99"
Cohesion: 0.40
Nodes (4): request_context(), Response, Next, Request

### Community 100 - "Community 100"
Cohesion: 0.40
Nodes (4): Model, Relation, String, Uuid

### Community 101 - "Community 101"
Cohesion: 0.50
Nodes (3): Protocol, StructuredCall, StructuredModel

### Community 102 - "Community 102"
Cohesion: 0.40
Nodes (4): main(), Box, Error, Result

### Community 104 - "Community 104"
Cohesion: 0.50
Nodes (3): rejects_invalid_request_semantics(), request(), Value

### Community 105 - "Community 105"
Cohesion: 0.70
Nodes (4): java_environment(), main(), Explicit local SDK setup and demo build; never runs checks or accepts SDK…, setup()

### Community 106 - "Community 106"
Cohesion: 0.50
Nodes (3): Model, Relation, Uuid

### Community 107 - "Community 107"
Cohesion: 0.83
Nodes (3): gradlew script, die(), warn()

### Community 109 - "Community 109"
Cohesion: 0.67
Nodes (3): main(), Explicit repository formatting; generators retain ownership of generated output., run()

### Community 110 - "Community 110"
Cohesion: 0.67
Nodes (3): install(), main(), Explicit project-local Android intake tool installation, never run at API…

### Community 112 - "Community 112"
Cohesion: 0.67
Nodes (3): migration, mobile-qa, mobile-qa-contracts

## Knowledge Gaps
- **176 isolated node(s):** `ErrorNoticeProps`, `PageHeadingProps`, `CoverageFlowModel`, `CoverageFlowStatus`, `CoverageNode` (+171 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 753 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **29 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `QualificationError` connect `Community 3` to `Community 2`, `Community 34`, `Community 4`, `Community 39`, `Community 9`, `Community 11`, `Community 15`, `Community 28`?**
  _High betweenness centrality (0.102) - this node is a cross-community bridge._
- **Why does `Session` connect `Community 8` to `Community 0`, `Community 1`, `Community 20`, `Community 45`?**
  _High betweenness centrality (0.085) - this node is a cross-community bridge._
- **Why does `WorkerModelCapabilities` connect `Community 18` to `Community 0`, `Community 12`, `Community 53`, `Community 15`?**
  _High betweenness centrality (0.067) - this node is a cross-community bridge._
- **Are the 57 inferred relationships involving `QualificationError` (e.g. with `DiscoveryBroker` and `BrokerClient`) actually correct?**
  _`QualificationError` has 57 INFERRED edges - model-reasoned connections that need verification._
- **Are the 57 inferred relationships involving `field()` (e.g. with `build()` and `create()`) actually correct?**
  _`field()` has 57 INFERRED edges - model-reasoned connections that need verification._
- **Are the 35 inferred relationships involving `Profile` (e.g. with `explore()` and `invoke()`) actually correct?**
  _`Profile` has 35 INFERRED edges - model-reasoned connections that need verification._
- **What connects `ErrorNoticeProps`, `PageHeadingProps`, `CoverageFlowModel` to the rest of the system?**
  _176 weakly-connected nodes found - possible documentation gaps or missing edges._