# Graph Report - mobile-qa  (2026-09-24)

## Corpus Check
- cluster-only mode — file stats not available

## Summary
- 3959 nodes · 10249 edges · 273 communities (176 shown, 81 thin omitted)
- Extraction: 95% EXTRACTED · 5% INFERRED · 0% AMBIGUOUS · INFERRED: 533 edges (avg confidence: 0.9)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `838ab875`
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
- Community 127
- Community 128
- Community 129
- Community 130
- Community 131
- Community 132
- Community 133
- Community 134
- Community 135
- Community 136
- Community 137
- Community 138
- Community 139
- Community 140
- Community 141
- Community 142
- Community 143
- Community 144
- Community 145
- Community 146
- Community 147
- Community 148
- Community 149
- Community 150
- Community 151
- Community 152
- Community 153
- Community 154
- Community 155
- Community 156
- Community 157
- Community 158
- Community 159
- Community 160
- Community 161
- Community 162
- Community 163
- Community 164
- Community 165
- Community 166
- Community 167
- Community 168
- Community 169
- Community 170
- Community 171
- Community 172
- Community 173
- Community 174
- Community 176
- Community 177
- Community 178
- Community 179
- Community 180
- Community 181
- Community 182
- Community 183
- Community 184
- Community 185
- Community 186
- Community 187
- Community 188
- Community 189
- Community 190
- Community 191
- Community 192
- Community 193
- Community 194
- Community 195
- Community 196
- Community 197
- Community 198
- Community 199
- Community 200
- Community 201
- Community 202
- Community 203
- Community 204
- Community 205
- Community 206
- Community 207
- Community 208
- Community 209
- Community 210
- Community 211
- Community 212
- Community 213
- Community 214
- Community 215
- Community 216
- Community 217
- Community 218
- Community 219
- Community 220
- Community 221
- Community 222
- Community 223
- Community 224
- Community 225
- Community 226
- Community 227
- Community 228
- Community 229
- Community 230
- Community 231
- Community 232
- Community 233
- Community 234
- Community 235
- Community 236
- Community 237
- Community 238
- Community 239
- Community 240
- Community 241
- Community 242
- Community 243
- Community 244
- Community 245
- Community 246
- Community 247
- Community 248
- Community 249
- Community 250
- Community 251
- Community 252
- Community 253
- Community 254
- Community 255
- Community 256
- Community 272

## God Nodes (most connected - your core abstractions)
1. `QualificationError` - 255 edges
2. `Profile` - 71 edges
3. `checked()` - 65 edges
4. `useWorkspace()` - 62 edges
5. `@mantine/core` - 60 edges
6. `Session` - 49 edges
7. `Evidence` - 47 edges
8. `decode()` - 47 edges
9. `@tanstack/react-query` - 47 edges
10. `AndroidDevice` - 43 edges

## Surprising Connections (you probably didn't know these)
- `qualify()` --uses--> `AndroidDevice`  [INFERRED]
  scripts/regression_acceptance.py → apps/mobile-worker/src/mobile_qa_worker/device/android.py
- `qualify()` --uses--> `Evidence`  [INFERRED]
  scripts/regression_acceptance.py → apps/mobile-worker/src/mobile_qa_worker/qualification/evidence.py
- `load()` --indirect_call--> `client()`  [INFERRED]
  apps/mobile-worker/src/mobile_qa_worker/artifacts/delivery.py → scripts/app_setup_smoke.py
- `task()` --indirect_call--> `client()`  [INFERRED]
  apps/mobile-worker/src/mobile_qa_worker/authoring/minitap_discovery.py → scripts/app_setup_smoke.py
- `create()` --indirect_call--> `profile()`  [INFERRED]
  scripts/test_library_smoke.py → apps/mobile-worker/tests/test_device.py

## Import Cycles
- None detected.

## Communities (273 total, 81 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.03
Nodes (98): Cache, installed_source(), Private, bounded APK cache; filesystem locks also fence concurrent processes.…, Pin immutable install bytes without a second multi-GiB copy per emulator., _authorized(), prepared_build(), check_preparation(), load() (+90 more)

### Community 1 - "Community 1"
Cohesion: 0.05
Nodes (120): action(), bearer(), cleanup(), grant(), heartbeat(), hint(), register(), ApiResult (+112 more)

### Community 2 - "Community 2"
Cohesion: 0.05
Nodes (112): change_plan(), checkout(), pilot(), quote(), ApiResult, AppContext, HeaderMap, Json (+104 more)

### Community 3 - "Community 3"
Cohesion: 0.06
Nodes (95): archive(), create(), default_plan(), draft(), entry(), list(), options(), ApiResult (+87 more)

### Community 4 - "Community 4"
Cohesion: 0.06
Nodes (94): artifact(), cancel(), case_create(), case_preview(), create(), detail(), history(), HistoryQuery (+86 more)

### Community 5 - "Community 5"
Cohesion: 0.04
Nodes (63): Sanitize the retained screen and hierarchy before reporting or model access., Owned Android operations, independent of app-specific qualification oracles., device_for(), Explicit app policies. Generic direct execution never uses the demo's backend…, execute(), png(), Path, Synthetic HTTP-worker evidence. It is never advertised as device qualification. (+55 more)

### Community 6 - "Community 6"
Cohesion: 0.07
Nodes (60): changeCreditPlan(), commercialAccessQuery(), createCreditCheckout(), cancelRun(), appQuery(), appsQuery(), createWorkspace(), App() (+52 more)

### Community 7 - "Community 7"
Cohesion: 0.06
Nodes (52): mocks, session, issue, attempt(), authentication, backendUnavailable, invalidPassword, options (+44 more)

### Community 8 - "Community 8"
Cohesion: 0.07
Nodes (67): doctor(), execute(), Path, Sequential approved actions on the qualified demo adapter, with supervised SDK…, sdk_action(), execute(), Supervised direct replay: seal clean-start proof before accepting a mutation., Explicit bounded navigation; imports the SDK after process environment… (+59 more)

### Community 9 - "Community 9"
Cohesion: 0.06
Nodes (61): b64(), client(), google_fixture(), google_sign_in(), main(), provision(), Secret-free HTTP acceptance using owned test accounts/processes and a synthetic…, Ephemeral RSA fixture keys; the API accepts these only in Environment::Test. (+53 more)

### Community 10 - "Community 10"
Cohesion: 0.05
Nodes (41): assert_ports_available(), Prove we can listen, without treating a closed TCP connection as a live owner., HostClient, UUID, HostConfig, Path, Operator-owned local configuration; no credentials or customer state belong…, SlotConfig (+33 more)

### Community 11 - "Community 11"
Cohesion: 0.05
Nodes (55): BrokerClient, check_context(), compact(), configure_agent(), bounded_model(), __call__(), direct_tool(), foreground() (+47 more)

### Community 12 - "Community 12"
Cohesion: 0.09
Nodes (52): ArtifactTransferError, partSha256(), uploadPartBytes(), abortMultipartUpload(), authorizeUploadPart(), completeMultipartUpload(), confirmUploadPart(), getMultipartUpload() (+44 more)

### Community 13 - "Community 13"
Cohesion: 0.08
Nodes (57): active_assignment(), advertise_execution(), advertise_phone(), eligible_models(), purpose_name(), register(), resolve(), resolve_for_new_work() (+49 more)

### Community 14 - "Community 14"
Cohesion: 0.05
Nodes (48): configure_auth(), MAX_APK, AppContext, Arc, Config, Environment, Option, PathBuf (+40 more)

### Community 15 - "Community 15"
Cohesion: 0.07
Nodes (37): DiscoveryBroker, Use the same pixel mask for retained evidence and provider requests., redact(), check(), device_rpc(), execute(), hierarchy(), nodes() (+29 more)

### Community 16 - "Community 16"
Cohesion: 0.07
Nodes (42): ApiError, ApkMetadata, AppListResponse, AppResponse, ApprovalStatus, AppSummary, BrowserApi, BuildListResponse (+34 more)

### Community 17 - "Community 17"
Cohesion: 0.07
Nodes (51): grant(), require_active_attempt(), ApiResult, Response, String, Uuid, stream(), streams_verified_bytes_and_retains_scratch_until_body_drop() (+43 more)

### Community 18 - "Community 18"
Cohesion: 0.06
Nodes (37): assert_peer(), Path, Bounded generated device messages on a mode-0600, same-UID Unix socket., _read(), read_frame(), request(), write_frame(), datetime (+29 more)

### Community 19 - "Community 19"
Cohesion: 0.09
Nodes (36): ArtifactError, _continue(), _digest(), _lock(), _lock_file(), _private_directory(), Path, UUID (+28 more)

### Community 20 - "Community 20"
Cohesion: 0.10
Nodes (41): openPhone(), cancelTestGeneration(), generateTests(), runPhoneCommand(), saveAuthoredTests(), templatesQuery(), PhoneWorkspace(), ResizableWorkspace() (+33 more)

### Community 21 - "Community 21"
Cohesion: 0.13
Nodes (50): String, summary(), verified_required_pass(), ActionKind, ApprovalPurpose, ArtifactRequest, AttemptResponse, CaseDefinition (+42 more)

### Community 22 - "Community 22"
Cohesion: 0.12
Nodes (23): ArtifactStore, local_storage_does_not_mint_remote_capabilities(), private_immutable_publication_and_symlink_rejection(), reserve_scratch(), ApiResult, Arc, DateTime, Drop (+15 more)

### Community 23 - "Community 23"
Cohesion: 0.14
Nodes (36): archiveLibraryEntry(), createLibraryEntry(), defaultPlanQuery(), libraryDraftQuery(), libraryEntryQuery(), libraryErrorDetails(), libraryHistoryQuery(), libraryKey() (+28 more)

### Community 24 - "Community 24"
Cohesion: 0.14
Nodes (37): AuthoringModelEnvelope, AuthoringModelRequest, AuthoringModelResponse, AuthoringUsage, AutomationSequence, CoverageKind, DirectCommand, DirectTarget (+29 more)

### Community 25 - "Community 25"
Cohesion: 0.08
Nodes (25): Observation, observe(), Deterministic oracle for the controlled persistence demo, not arbitrary…, validate_png(), verdict(), parametrize, test_campaign_oracle_never_reaches_runner(), run() (+17 more)

### Community 26 - "Community 26"
Cohesion: 0.27
Nodes (35): abort_multipart(), authorize_part(), complete_multipart(), complete_upload(), confirm_part(), create_app(), create_upload(), create_workspace() (+27 more)

### Community 27 - "Community 27"
Cohesion: 0.15
Nodes (29): archive(), archive_crc_integrity_and_abi_inventory(), attr(), checksum_mismatch_is_infrastructure_not_invalid_apk(), directory(), inspect(), inspect_archive(), Inspection (+21 more)

### Community 28 - "Community 28"
Cohesion: 0.10
Nodes (28): attemptStatus(), buildLibraryCoverageFlow(), buildRunCoverageFlow(), caseCount(), caseVersion(), choiceLabel(), CoverageCanvasEditor(), CoverageFlowModel (+20 more)

### Community 29 - "Community 29"
Cohesion: 0.11
Nodes (27): canonical_query(), client(), completion_http_200_error_is_never_success(), CONTROL_RESPONSE_LIMIT, download_capability_has_no_upload_authority(), HASH, KEY, MAX_PARTS (+19 more)

### Community 30 - "Community 30"
Cohesion: 0.10
Nodes (17): AppRoutes, App, AppContext, Box, Config, Environment, Path, Result (+9 more)

### Community 31 - "Community 31"
Cohesion: 0.11
Nodes (22): Device, parametrize, Deterministic device seam; no emulator, network or model credentials., Rpc, test_cancel_before_action_has_no_effect(), test_capture_shares_active_rpc_instead_of_starting_a_second_instrumentation(), test_literal_set_text_uses_rpc_without_model(), test_rejects_ambiguous_disabled_secret_outside_targets() (+14 more)

### Community 32 - "Community 32"
Cohesion: 0.15
Nodes (18): createCommercialCheckQuote(), caseRunPreviewQuery(), createCaseRun(), createSuiteRun(), historyQuery(), suiteRunPreviewQuery(), runQuery(), phoneOptionsQuery() (+10 more)

### Community 33 - "Community 33"
Cohesion: 0.28
Nodes (27): build(), BuildQuery, claim(), cleanup(), complete(), delivery(), events(), heartbeat() (+19 more)

### Community 34 - "Community 34"
Cohesion: 0.15
Nodes (22): compare(), compares_verified_facts_and_freezes_baseline_identity(), context_matches(), coverage_changes_remain_visible(), eligible(), failed(), manifest_legacy_serialization_is_unchanged(), outcome() (+14 more)

### Community 35 - "Community 35"
Cohesion: 0.09
Nodes (22): name, packageManager, private, type, version, eslint, @eslint/js, globals (+14 more)

### Community 36 - "Community 36"
Cohesion: 0.13
Nodes (13): adapter(), assigned(), case_matches(), duration(), ApiResult, ConnectionTrait, String, Uuid (+5 more)

### Community 37 - "Community 37"
Cohesion: 0.33
Nodes (23): decode(), json(), ApiResult, T, Value, authorized(), claim(), detail() (+15 more)

### Community 38 - "Community 38"
Cohesion: 0.14
Nodes (21): google_identity_binding_expiry_replay_and_password_removal(), public_registration_approval_and_multiple_workspace_isolation(), challenge(), configure_google(), DATABASE_BOOT, GOOGLE_CLIENT, google_keys(), google_login() (+13 more)

### Community 39 - "Community 39"
Cohesion: 0.28
Nodes (14): expired(), field(), hmac(), multipart_control_lifecycle_uses_bounded_fake_provider(), MultipartClient, parse_xml(), ApiResult, Option (+6 more)

### Community 40 - "Community 40"
Cohesion: 0.11
Nodes (6): One-device operator qualification; no production scheduler or customer…, UsageRecorder, model(), test_qualification_rejects_non_navigation_model_before_provider_setup(), test_sdk_exact_public_seam(), test_usage_deduplicates_and_keeps_unknown()

### Community 41 - "Community 41"
Cohesion: 0.10
Nodes (14): controls(), goal_for(), Only bounded non-password controls can become selectable model context., Re-resolve element identity; old screen coordinates never authorize a blind tap., parametrize, Synthetic screen context only; no emulator, model or network access., test_controls_filter_passwords_and_preserve_bounds(), test_goal_uses_current_control_and_rejects_missing_identity() (+6 more)

### Community 42 - "Community 42"
Cohesion: 0.19
Nodes (17): backfill(), create_schema(), Migration, Model, referenced_ids(), Relation, review_event_id(), DateTimeUtc (+9 more)

### Community 43 - "Community 43"
Cohesion: 0.32
Nodes (20): authorized(), create(), detail(), environment(), list(), ListQuery, membership(), origins() (+12 more)

### Community 44 - "Community 44"
Cohesion: 0.20
Nodes (18): acknowledge(), recorded(), require(), ApiResult, AppContext, ConnectionTrait, Uuid, ExecutionContextV1 (+10 more)

### Community 45 - "Community 45"
Cohesion: 0.29
Nodes (19): assemble(), attempt(), authorize(), cancel(), create(), create_with_quote(), detail(), list() (+11 more)

### Community 46 - "Community 46"
Cohesion: 0.26
Nodes (20): claim(), claim_wait(), cleanup(), complete(), events(), heartbeat(), lease(), LeaseRecord (+12 more)

### Community 47 - "Community 47"
Cohesion: 0.10
Nodes (20): compilerOptions, allowImportingTsExtensions, baseUrl, esModuleInterop, jsx, lib, module, moduleResolution (+12 more)

### Community 48 - "Community 48"
Cohesion: 0.16
Nodes (18): ContractProbe, FakeExecutionRequest, FakeExecutionResult, nullable_string_schema(), Outcome, required_nullable(), DateTime, Error (+10 more)

### Community 49 - "Community 49"
Cohesion: 0.38
Nodes (18): build(), claim(), delivery(), detail(), open(), options(), ApiResult, AppContext (+10 more)

### Community 50 - "Community 50"
Cohesion: 0.23
Nodes (19): DEADLINE_SECONDS, heartbeat(), LEASE_SECONDS, ApiResult, AppContext, Model, Path, String (+11 more)

### Community 51 - "Community 51"
Cohesion: 0.16
Nodes (17): Explicit bounded AI authoring; ordinary device operations never enter this…, call(), parametrize, Discovery guards with synthetic device effects; the separate SDK fixture uses…, seam(), task(), test_budget_reserves_one_drafting_call_and_counts_unfinished_calls(), test_changed_foreground_and_oversized_context_fail_closed() (+9 more)

### Community 52 - "Community 52"
Cohesion: 0.32
Nodes (19): OpenPhoneRequest, PhoneBuildChoice, PhoneClaimRequest, PhoneClaimResponse, PhoneControl, PhoneFrame, PhoneLease, PhoneOptions (+11 more)

### Community 53 - "Community 53"
Cohesion: 0.21
Nodes (16): arg(), execute(), id(), Operator, ApiResult, AppContext, DateTime, Result (+8 more)

### Community 54 - "Community 54"
Cohesion: 0.23
Nodes (18): assertion_absence_requires_a_ready_unambiguous_screen(), clean_start_route_is_fenced_and_recovery_keeps_original_evidence(), comparison_finalization_is_durable_idempotent_and_baseline_is_pinned(), definition(), evidence_http_roundtrip_pass_failure_and_blocked(), evidence_verifier_distinguishes_missing_task_from_missing_prerequisite(), long_poll_rechecks_revocation_after_waking(), long_poll_wakes_on_committed_run_and_times_out_without_reserving() (+10 more)

### Community 55 - "Community 55"
Cohesion: 0.16
Nodes (13): execute(), parse_request(), Return the result chosen by a fixture; do not discover whether an app works.…, Validate fixture JSON without coercing values such as string versions to…, Echo the run ID and map the requested scenario to its fixed simulated outcome.…, parametrize, Python half of the shared fixture contract and local CLI checks. No device SDK…, test_cli_invalid_nonzero() (+5 more)

### Community 56 - "Community 56"
Cohesion: 0.11
Nodes (18): devDependencies, eslint, @eslint/js, globals, @hey-api/openapi-ts, jsdom, prettier, @testing-library/jest-dom (+10 more)

### Community 57 - "Community 57"
Cohesion: 0.29
Nodes (16): action(), cancel(), command(), enqueue(), generate(), job_detail(), ApiResult, AppContext (+8 more)

### Community 58 - "Community 58"
Cohesion: 0.29
Nodes (16): base(), concurrent_saves_scope_csrf_and_actual_route_contracts(), create(), failure(), legacy_review_states_are_editable_and_old_receipts_fail_closed(), members_save_cases_suites_and_plans_without_grants_and_pins_stay_fixed(), newer_saved_formats_do_not_break_the_catalog(), AppContext (+8 more)

### Community 59 - "Community 59"
Cohesion: 0.42
Nodes (15): build(), build_response(), complete(), create(), list(), ApiResult, AppContext, ListQuery (+7 more)

### Community 60 - "Community 60"
Cohesion: 0.15
Nodes (15): Fixture, FixtureState, receive(), RecordedRequest, Arc, Body, Drop, IntoResponse (+7 more)

### Community 61 - "Community 61"
Cohesion: 0.23
Nodes (13): arg(), execute(), Execution, id(), read(), ApiResult, AppContext, Result (+5 more)

### Community 62 - "Community 62"
Cohesion: 0.24
Nodes (13): every_declared_route_requires_the_declared_security_and_errors(), persisted_workflow_auth_scope_and_real_validation(), rejection_recovery_and_infrastructure_failure(), throttles_quota_and_revision_scope(), execution_route_inventory_has_actual_auth_and_errors(), expired_validation_attempt_recovers_from_durable_job(), immutable_part_fingerprints_and_completion_are_fenced(), inclusive_size_metadata_and_multipart_auth_boundaries() (+5 more)

### Community 63 - "Community 63"
Cohesion: 0.19
Nodes (14): expired_agreement_can_be_closed_before_a_new_period(), next_plan_grants_only_on_paid_renewal_and_old_invoice_replay_is_stable(), paid_period_holds_credits_atomically_and_queued_cancel_releases_them(), pilot_quote_authorization_replay_cancel_credit_and_tenant_isolation(), prepared(), recurring_price_is_server_owned_and_period_begins_with_zero_checks(), AppContext, TestServer (+6 more)

### Community 64 - "Community 64"
Cohesion: 0.34
Nodes (13): cleanup_pending(), content(), record(), reserve(), ApiResult, AppContext, Body, Model (+5 more)

### Community 66 - "Community 66"
Cohesion: 0.24
Nodes (8): assert_nested_virtualization(), check(), offline_environment(), provider_schema_configuration(), Explicit configuration validation; never apply, plan against AWS, build AMIs or…, Keep the exact version/provider requirements, excluding operational backend.…, DeploymentCheckTests, No network or tools: verify credential removal and the provider schema gate.

### Community 67 - "Community 67"
Cohesion: 0.19
Nodes (10): Migration, Model, Relation, DbErr, MigrationTrait, Option, Result, SchemaManager (+2 more)

### Community 68 - "Community 68"
Cohesion: 0.29
Nodes (10): Dispatcher, ApiResult, AppContext, Option, Self, String, Uuid, sign() (+2 more)

### Community 69 - "Community 69"
Cohesion: 0.50
Nodes (12): get(), import(), operator(), profile(), register_profile(), require_case(), resolve(), ApiResult (+4 more)

### Community 70 - "Community 70"
Cohesion: 0.24
Nodes (11): lease_authority(), register(), require_live_host_grant(), ApiResult, AppContext, ConnectionTrait, Model, Parts (+3 more)

### Community 71 - "Community 71"
Cohesion: 0.32
Nodes (6): android.app.Activity, android.os.Bundle, android.widget.LinearLayout, MainActivity, LinearLayout, Override

### Community 73 - "Community 73"
Cohesion: 0.18
Nodes (4): Offline policy construction; never installs nft rules or runs a privileged…, test_failed_kernel_policy_check_does_not_publish_ready_marker(), run(), test_successful_policy_install_publishes_marker_only_after_atomic_apply()

### Community 74 - "Community 74"
Cohesion: 0.17
Nodes (12): dependencies, lucide-react, @mantine/core, @mantine/form, @mantine/hooks, react, react-dom, @react-oauth/google (+4 more)

### Community 75 - "Community 75"
Cohesion: 0.26
Nodes (6): healthQuery, apiClient, ApiClientError, requestMethods, transportClient, healthy

### Community 76 - "Community 76"
Cohesion: 0.26
Nodes (9): files(), main(), Path, Generate both consumer contracts from Rust without starting the application.…, Read managed output bytes keyed by repo-relative path, independent of mtimes., Compare the whole generated set, including new and removed files. Preserve…, synchronize(), ExportTests (+1 more)

### Community 77 - "Community 77"
Cohesion: 0.25
Nodes (10): HistoryRecord, list(), ApiResult, AppContext, DateTime, Option, String, Utc (+2 more)

### Community 78 - "Community 78"
Cohesion: 0.25
Nodes (10): evaluate(), observe(), png_valid(), ApiResult, AppContext, Outcome, Result, String (+2 more)

### Community 79 - "Community 79"
Cohesion: 0.22
Nodes (8): Cleanup, execute(), ApiResult, AppContext, Result, Task, TaskInfo, Vars

### Community 80 - "Community 80"
Cohesion: 0.35
Nodes (10): assignment_revisions_change_only_future_resolution(), definition(), insert_attempt(), insert_run(), multi_reference_capabilities_are_bounded_and_exact(), recovery_queue_links_only_to_a_run_in_the_same_app(), registry_is_immutable_resolvable_and_retirable(), ConnectionTrait (+2 more)

### Community 81 - "Community 81"
Cohesion: 0.18
Nodes (10): binaryUpload, completeStatuses, editableDefinition, healthy, implicitDefault, persistedBuild, staleBuild, staleResponse (+2 more)

### Community 82 - "Community 82"
Cohesion: 0.29
Nodes (7): composite_foreign_key(), Migration, DbErr, MigrationTrait, Result, SchemaManager, TableForeignKey

### Community 83 - "Community 83"
Cohesion: 0.24
Nodes (8): ExecutionWakeup, notify(), AppContext, Self, subscribe(), Default, Receiver, Sender

### Community 84 - "Community 84"
Cohesion: 0.20
Nodes (4): OwnedDevice, Path, Protocol, UUID

### Community 87 - "Community 87"
Cohesion: 0.42
Nodes (6): create_receipt_table(), Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 88 - "Community 88"
Cohesion: 0.25
Nodes (3): DeviceRpc, ElementRpc, Protocol

### Community 89 - "Community 89"
Cohesion: 0.22
Nodes (9): scripts, build, dev, format, format:check, generate:sdk, lint, test (+1 more)

### Community 92 - "Community 92"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 93 - "Community 93"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 94 - "Community 94"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 95 - "Community 95"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 96 - "Community 96"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 97 - "Community 97"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 98 - "Community 98"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 99 - "Community 99"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 100 - "Community 100"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 101 - "Community 101"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 102 - "Community 102"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 103 - "Community 103"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 104 - "Community 104"
Cohesion: 0.36
Nodes (5): Migration, DbErr, MigrationTrait, Result, SchemaManager

### Community 105 - "Community 105"
Cohesion: 0.25
Nodes (7): Model, Relation, DateTimeUtc, Json, Option, String, Uuid

### Community 106 - "Community 106"
Cohesion: 0.25
Nodes (7): Model, Relation, DateTimeUtc, Json, Option, String, Uuid

### Community 107 - "Community 107"
Cohesion: 0.25
Nodes (7): Model, Relation, DateTimeUtc, Json, Option, String, Uuid

### Community 108 - "Community 108"
Cohesion: 0.25
Nodes (7): Model, Relation, DateTimeUtc, Json, Option, String, Uuid

### Community 109 - "Community 109"
Cohesion: 0.25
Nodes (7): Model, Relation, DateTimeUtc, Json, Option, String, Uuid

### Community 110 - "Community 110"
Cohesion: 0.25
Nodes (7): Model, Relation, DateTimeUtc, Json, Option, String, Uuid

### Community 111 - "Community 111"
Cohesion: 0.25
Nodes (7): Model, Relation, DateTimeUtc, Json, Option, String, Uuid

### Community 112 - "Community 112"
Cohesion: 0.25
Nodes (7): Model, Relation, DateTimeUtc, Json, Option, String, Uuid

### Community 113 - "Community 113"
Cohesion: 0.36
Nodes (5): installer(), parametrize, Exercise the pinned installer with local archives only; never contact Google., test_install_checks_archive_and_preserves_managed_metadata(), test_installer_only_selects_native_system_image()

### Community 117 - "Community 117"
Cohesion: 0.39
Nodes (5): approved_import_shape_has_separate_actions_and_checks(), case(), invalid_definition_references_and_paths_are_rejected(), manual_methods_are_defined_but_not_implicitly_executable(), unknown_fields_are_not_imported()

### Community 118 - "Community 118"
Cohesion: 0.29
Nodes (5): Migrator, Box, MigrationTrait, Vec, MigratorTrait

### Community 119 - "Community 119"
Cohesion: 0.43
Nodes (5): current(), method_not_allowed(), Json, StatusCode, unknown()

### Community 120 - "Community 120"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 121 - "Community 121"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 122 - "Community 122"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 123 - "Community 123"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 124 - "Community 124"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 125 - "Community 125"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 126 - "Community 126"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 127 - "Community 127"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 128 - "Community 128"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 129 - "Community 129"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Json, String, Uuid

### Community 130 - "Community 130"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 131 - "Community 131"
Cohesion: 0.29
Nodes (6): Model, Relation, Json, Option, String, Uuid

### Community 132 - "Community 132"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 133 - "Community 133"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 134 - "Community 134"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Json, String, Uuid

### Community 135 - "Community 135"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Json, String, Uuid

### Community 136 - "Community 136"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Json, String, Uuid

### Community 137 - "Community 137"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 138 - "Community 138"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Json, Option, Uuid

### Community 139 - "Community 139"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 140 - "Community 140"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Json, String, Uuid

### Community 141 - "Community 141"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 142 - "Community 142"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 143 - "Community 143"
Cohesion: 0.29
Nodes (6): Model, Relation, DateTimeUtc, Option, String, Uuid

### Community 144 - "Community 144"
Cohesion: 0.43
Nodes (6): discovery_progress(), ApiResult, Option, T, same(), task_update()

### Community 145 - "Community 145"
Cohesion: 0.67
Nodes (4): startGoogleSignIn(), useGoogleSignIn(), safeReturnTo(), SignIn()

### Community 146 - "Community 146"
Cohesion: 0.48
Nodes (6): fixture(), optional_null_is_accepted_then_omitted(), probe_roundtrips_without_losing_precision(), rejects_invalid_probe_boundaries(), Value, scenario_and_version_are_validated()

### Community 147 - "Community 147"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 148 - "Community 148"
Cohesion: 0.33
Nodes (5): Model, Relation, Option, String, Uuid

### Community 149 - "Community 149"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 150 - "Community 150"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 151 - "Community 151"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, Option, Uuid

### Community 152 - "Community 152"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 153 - "Community 153"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 154 - "Community 154"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 155 - "Community 155"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 156 - "Community 156"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 157 - "Community 157"
Cohesion: 0.33
Nodes (5): Model, Relation, Option, String, Uuid

### Community 158 - "Community 158"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 159 - "Community 159"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 160 - "Community 160"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 161 - "Community 161"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 162 - "Community 162"
Cohesion: 0.33
Nodes (5): Model, Relation, DateTimeUtc, String, Uuid

### Community 163 - "Community 163"
Cohesion: 0.47
Nodes (5): create(), ApiResult, AppContext, Uuid, CreateWorkspaceRequest

### Community 164 - "Community 164"
Cohesion: 0.40
Nodes (5): backend_has_no_raw_sql_escape_hatches(), Path, PathBuf, Vec, rust_files()

### Community 166 - "Community 166"
Cohesion: 0.40
Nodes (4): request_context(), Request, Response, Next

### Community 167 - "Community 167"
Cohesion: 0.40
Nodes (4): Model, Relation, DateTimeUtc, Uuid

### Community 168 - "Community 168"
Cohesion: 0.40
Nodes (4): Model, Relation, Json, Uuid

### Community 169 - "Community 169"
Cohesion: 0.40
Nodes (4): Model, Relation, Json, Uuid

### Community 170 - "Community 170"
Cohesion: 0.40
Nodes (4): Model, Relation, String, Uuid

### Community 171 - "Community 171"
Cohesion: 0.40
Nodes (4): Model, Relation, String, Uuid

### Community 172 - "Community 172"
Cohesion: 0.40
Nodes (4): Model, Relation, DateTimeUtc, Uuid

### Community 173 - "Community 173"
Cohesion: 0.50
Nodes (3): Protocol, StructuredCall, StructuredModel

### Community 174 - "Community 174"
Cohesion: 0.40
Nodes (4): main(), Box, Error, Result

### Community 176 - "Community 176"
Cohesion: 0.50
Nodes (3): rejects_invalid_request_semantics(), request(), Value

### Community 177 - "Community 177"
Cohesion: 0.70
Nodes (4): java_environment(), main(), Explicit local SDK setup and demo build; never runs checks or accepts SDK…, setup()

### Community 178 - "Community 178"
Cohesion: 0.50
Nodes (3): Model, Relation, Uuid

### Community 179 - "Community 179"
Cohesion: 0.50
Nodes (3): Model, Relation, Uuid

### Community 180 - "Community 180"
Cohesion: 0.50
Nodes (3): Entity, Related, RelationDef

### Community 181 - "Community 181"
Cohesion: 0.50
Nodes (3): Entity, Related, RelationDef

### Community 182 - "Community 182"
Cohesion: 0.67
Nodes (3): finalize_pending(), ApiResult, AppContext

### Community 183 - "Community 183"
Cohesion: 0.83
Nodes (3): gradlew script, die(), warn()

### Community 186 - "Community 186"
Cohesion: 0.67
Nodes (3): main(), Explicit repository formatting; generators retain ownership of generated output., run()

### Community 187 - "Community 187"
Cohesion: 0.67
Nodes (3): install(), main(), Explicit project-local Android intake tool installation, never run at API…

### Community 190 - "Community 190"
Cohesion: 0.67
Nodes (3): migration, mobile-qa, mobile-qa-contracts

## Knowledge Gaps
- **302 isolated node(s):** `Progress`, `CoverageFlowModel`, `CoverageFlowStatus`, `CoverageNode`, `CoverageNodeData` (+297 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 1290 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **81 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `QualificationError` connect `Community 15` to `Community 0`, `Community 5`, `Community 8`, `Community 72`, `Community 10`, `Community 11`, `Community 41`, `Community 40`, `Community 18`, `Community 51`, `Community 25`, `Community 31`?**
  _High betweenness centrality (0.096) - this node is a cross-community bridge._
- **Why does `WorkerModelCapabilities` connect `Community 13` to `Community 0`, `Community 48`, `Community 52`, `Community 46`?**
  _High betweenness centrality (0.055) - this node is a cross-community bridge._
- **Why does `worker_capabilities()` connect `Community 0` to `Community 8`, `Community 11`, `Community 13`?**
  _High betweenness centrality (0.051) - this node is a cross-community bridge._
- **Are the 83 inferred relationships involving `QualificationError` (e.g. with `DiscoveryBroker` and `BrokerClient`) actually correct?**
  _`QualificationError` has 83 INFERRED edges - model-reasoned connections that need verification._
- **Are the 43 inferred relationships involving `Profile` (e.g. with `explore()` and `invoke()`) actually correct?**
  _`Profile` has 43 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Progress`, `CoverageFlowModel`, `CoverageFlowStatus` to the rest of the system?**
  _302 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.034245187436676795 - nodes in this community are weakly interconnected._