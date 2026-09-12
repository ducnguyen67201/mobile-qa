# Implementation report: Phase 02 Android qualification harness

## Outcome

The operator harness is implemented and passes offline validation on
`feat/02-cloud-phone-and-feasibility`, based on merged `main` at `d137e35`.
It adds a controlled Android good/broken demo, an independent persistence verifier,
private evidence, bounded SDK subprocess execution, reset/quarantine handling and
an eleven-attempt qualification campaign. Existing fake/import behavior is preserved.

**Phase 02 is not yet qualified.** No cloud host was provisioned, no emulator was booted,
and no agent/model call occurred. Task 12 requires actual host/model inputs, a concrete
launch and spend packet, authorization, live evidence, verified transfer and teardown.
The [plan](../plans/02-cloud-phone-and-feasibility.plan.md) stays unarchived. Product and
operational authority remains [docs/architect](../../../docs/architect/README.md).

## Assessment against plan

| Item | Planned | Observed |
|---|---|---|
| Complexity | XL, authoring/offline/live gates | Matches; device reliability still unknown |
| Confidence | 8/10 implementation estimate | Offline checks pass; no numerical live reliability claim |
| Files | Approximately 50–55 | 61 (42 created, 19 updated), including this report and the original plan |
| Architecture | Rust contracts; Python adapter; isolated Android fixture | Preserved; no new API/ORM/UI feature or second agent framework |
| Dependency changes | Existing pinned Minitap and callback seam | Minitap 4.0.0 unchanged; langchain-core 1.6.3 promoted from existing transitive resolution |

## Task status

| Task | Status | Evidence / limit |
|---|---|---|
| 1 Scope, host/toolchain inputs | Code ready; live choices pending | Official archive URLs/checksums/revisions pinned; unresolved region/AMI/model remain explicit |
| 2 Qualification wire contracts | Implemented | Rust source, generated schema/Pydantic and semantic checks |
| 3 Controlled demo and backend | Implemented; APKs built | Same package/UI; durable good build and memory-only broken build; actual loopback HTTP test |
| 4 Device preflight/reset | Implemented; offline tested | Pinned serial/profile, APK checks, fresh owned AVD; real boot/reset unverified |
| 5 SDK boundary and usage | Implemented; offline tested | Explicit profile, Doppler child, import isolation, callbacks and process ownership |
| 6 Verifier and evidence | Implemented; offline tested | Exact task row, create/reopen XML/PNG, hashes, write-once results and missing reasons |
| 7 Supervisor and CLI | Implemented; offline tested | Doctor/run/recover commands, locking, dirty state, bounded cleanup and quarantine |
| 8 Campaign and measurement | Implemented; offline tested | Nine scenarios, interruption, fresh good attempt; mismatch rejects; unknown costs remain unknown |
| 9 Commands and CI | Implemented; selection tested | Explicit device recipes; demo/worker filters; no emulator/model in ordinary checks |
| 10 Canonical docs | Updated | Runbook, status, provenance, environment and source ownership |
| 11 Offline verification | Passed | Commands below; corrections were batched, failed/invalidated checks rerun |
| 12 Authorized cloud qualification | Pending | No real classification, reset, cancellation, cost or transfer evidence yet |

## Validation performed

| Check | Result |
|---|---|
| `just types` | Generated only worker schema and Python consumer changes; browser outputs unchanged |
| `cargo fmt --package mobile-qa-contracts -- --check` | Passed after formatting |
| Contract Clippy with `--all-targets --locked -- -D warnings` | Passed |
| `cargo test --package mobile-qa-contracts --locked` | 6 passed: 4 existing, 2 qualification tests |
| Worker Ruff | Passed |
| Pre-commit installed-worker dependency audit | No known vulnerabilities; local editable worker excluded from registry lookup |
| Worker Pyright from its project directory | 0 errors, 0 warnings; strict mode retained |
| Worker pytest | 80 passed: 26 existing, 54 new |
| `just check-contracts` | 0 drift; exporter synchronization test passed |
| `uv build --project apps/mobile-worker` | Wheel and source distribution built |
| `scripts/sdk_smoke.py` | Minitap 4.0.0 imported with socket connections blocked; no Agent constructed |
| Android good/broken debug assemble | Both passed with JDK 17, pinned SDK and Gradle wrapper |
| Android `lintGoodDebug` and `lintBrokenDebug` | Both passed; fixture/version warnings remain, no baseline hiding errors |
| APK metadata inspection using pinned `aapt` | Both package/activity/minSdk/targetSdk verified; no native libraries |
| Actual CI filter regression script | 10 path cases passed, including combined/deletion paths |
| Device recipe dry runs / setup shell syntax | Passed; explicit arguments required, no device execution |
| Diff and documentation consistency | Reviewed; generated output and lock changes limited to intended worker scope |

Full API/web suites were not rerun because their runtime source did not change. Shared
workflow changes will select the configured broader hosted jobs when this branch is
pushed; hosted CI has not run for this implementation. Local tests are not Linux/KVM
acceptance. Python SDK import warnings and Android fixed-version/fixture lint warnings
are retained as upstream or deliberate fixture limitations, not suppressed wholesale.

## Tests added

| File | Collected cases | Main coverage |
|---|---:|---|
| `test_qualification.py` | 18 | Strict request/result boundaries, immutable private evidence, path containment, limits, unsupported-host doctor |
| `test_verifier.py` | 12 | Persistence decision table, unavailable/ambiguous observations, exact package/task and invalid evidence |
| `test_device.py` | 7 | Credential isolation, busy host, subprocess timeout/output limit, orphan descendant cleanup, APK compatibility, unsafe deletion |
| `test_sdk_adapter.py` | 3 | Callback deduplication/unknown usage, environment preparation, actual public SDK method configuration seam |
| `test_campaign.py` | 10 | Supervisor good/broken/blocked/cancel/quarantine outcomes, same-boot recovery refusal, HTTP fixture, campaign oracle isolation |
| `test_install_tools.py` | 4 | Namespaced pinned metadata, local archive checksum/traversal handling, managed reinstall and overwrite refusal |
| Rust `qualification.rs` tests | 2 | Invalid request semantics and refusal of decisive results without evidence |

## Built demo artifacts

These are local debug artifacts, excluded from Git, not customer distributions.
Both use `ai.mobileqa.demo`, launch `ai.mobileqa.demo.MainActivity`, minSdk 26 and target 35.

| Flavor | Bytes | SHA256 |
|---|---:|---|
| good | 11833 | `06ea251a975862982230c9055863c89a17eb1c2f7c2458677b30be3625209caf` |
| broken | 11701 | `b510f4d969fe992d20e48066f987c5fecfb432a94e84538ec6dec14236f49400` |

Paths: `apps/qa-demo-android/app/build/outputs/apk/{good,broken}/debug/`.
Rebuilt/debug-signed APK hashes can differ; each request verifies the actual supplied hash.

## Deviations and fixes

- Crash recovery requires a changed host boot ID before removing stale owned AVD state.
  This deliberately avoids killing guessed PIDs or reconnecting capacity while an old
  SDK might survive. Ordinary cancellation first stops the owned group and verifies reset.
- Cleanup defaults to 240 seconds, rather than the plan's initial 90-second suggestion,
  because it performs a complete second cold boot/install/reset proof. Runtime budgets
  remain bounded and will need measurement on the chosen host.
- Small `process.py` and `commands.py` modules isolate process lifecycle and lazy CLI
  dispatch; no framework or repository abstraction was introduced.
- Vendor archive checksums use the SHA1 supplied by Google's repository metadata;
  run/APK/artifact evidence uses SHA256. The installer preserves namespace/license
  metadata and does not create SDK license-acceptance files.
- Demo-only installation includes pinned platform tools because Gradle otherwise tries
  to resolve them separately. Android lint now names both flavors explicitly: generic
  `lint` selected only the first debug flavor in the observed build.
- Process termination inspection handles exited parents with surviving descendants and
  macOS zombie/permission behavior. Unconfirmed SDK termination forces quarantine even
  if a business assertion would otherwise pass. Command output is capped during reads.
- Fixed the reset shell quoting, SDK tool path consistency, AVD duplicate configuration
  keys, and fixture startup ownership marker before declaring offline readiness.
- Evidence transfer, total billing reconciliation and host shutdown remain explicit
  operator steps in the live runbook. No production object-store/upload service was added.
  Completed phase timings and UTC attempt boundaries are recorded; interrupted/missing
  phase measurements and provider usage must remain gaps, never invented zeros.

## Remaining live gate

Provide an existing Linux x86_64 KVM host or choose a region for the AWS candidate,
then resolve actual AMI/network/access, accepted SDK terms, the approved model and
Doppler credential, current pricing, spend and shutdown deadline. The tracked launch
JSON is a review template, not a concrete or authorized cloud request. Execute all
nine expected scenarios plus interruption/recovery, retain every mismatch, copy and
verify evidence, record full costs/timings, and confirm resource shutdown.

Phase 03 app setup may proceed independently. Phase 04 real execution and phase 02
completion remain gated on that evidence. No customer or production readiness is claimed.

## Hosting follow-up

The user selected Railway for initial API/dashboard/database hosting and AWS as a later
migration target. [Hosting](../../../docs/architect/hosting.md) records the decision;
the KVM device-host gate stays open and no deployment was performed.

## Changed files

Counts include source, generated artifacts, fixtures, infrastructure, plan and report.
Line counts are final file sizes, not added-line counts; the report's own count is omitted.

| File | Action | Lines |
|---|---|---:|
| `.claude/PRPs/plans/02-cloud-phone-and-feasibility.plan.md` | Created | 483 |
| `.claude/PRPs/reports/02-cloud-phone-and-feasibility-report.md` | Created | — |
| `.github/workflows/ci.yaml` | Updated | 215 |
| `.gitignore` | Updated | 16 |
| `AGENTS.md` | Updated | 31 |
| `apps/mobile-worker/pyproject.toml` | Updated | 39 |
| `apps/mobile-worker/src/mobile_qa_worker/cli.py` | Updated | 84 |
| `apps/mobile-worker/src/mobile_qa_worker/generated/models.py` | Updated | 183 |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/__init__.py` | Created | 1 |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/campaign.py` | Created | 241 |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/commands.py` | Created | 40 |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/config.py` | Created | 95 |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/device.py` | Created | 338 |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/evidence.py` | Created | 154 |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/process.py` | Created | 116 |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/runner.py` | Created | 327 |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/sdk_adapter.py` | Created | 173 |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/verifier.py` | Created | 110 |
| `apps/mobile-worker/tests/test_campaign.py` | Created | 169 |
| `apps/mobile-worker/tests/test_device.py` | Created | 105 |
| `apps/mobile-worker/tests/test_install_tools.py` | Created | 65 |
| `apps/mobile-worker/tests/test_qualification.py` | Created | 164 |
| `apps/mobile-worker/tests/test_sdk_adapter.py` | Created | 118 |
| `apps/mobile-worker/tests/test_verifier.py` | Created | 72 |
| `apps/mobile-worker/uv.lock` | Updated | 3449 |
| `apps/qa-demo-android/app/build.gradle` | Created | 14 |
| `apps/qa-demo-android/app/src/main/AndroidManifest.xml` | Created | 13 |
| `apps/qa-demo-android/app/src/main/java/ai/mobileqa/demo/MainActivity.java` | Created | 97 |
| `apps/qa-demo-android/app/src/main/res/values/ids.xml` | Created | 8 |
| `apps/qa-demo-android/app/src/main/res/xml/network_security_config.xml` | Created | 5 |
| `apps/qa-demo-android/build.gradle` | Created | 2 |
| `apps/qa-demo-android/gradle.properties` | Created | 2 |
| `apps/qa-demo-android/gradle/wrapper/gradle-wrapper.jar` | Created | binary |
| `apps/qa-demo-android/gradle/wrapper/gradle-wrapper.properties` | Created | 8 |
| `apps/qa-demo-android/gradlew` | Created | 251 |
| `apps/qa-demo-android/gradlew.bat` | Created | 94 |
| `apps/qa-demo-android/settings.gradle` | Created | 4 |
| `contracts/worker.schema.json` | Updated | 461 |
| `crates/contracts/src/worker.rs` | Updated | 132 |
| `crates/contracts/src/worker/qualification.rs` | Created | 184 |
| `crates/contracts/tests/qualification.rs` | Created | 56 |
| `docs/architect/README.md` | Updated | 52 |
| `docs/architect/decisions.md` | Updated | 26 |
| `docs/architect/dependencies.md` | Updated | 56 |
| `docs/architect/development.md` | Updated | 92 |
| `docs/architect/device-qualification.md` | Created | 142 |
| `docs/architect/environment.md` | Updated | 87 |
| `docs/architect/hosting.md` | Created | 79 |
| `docs/architect/implementation/00-master-spec.md` | Updated | 133 |
| `docs/architect/implementation/02-cloud-phone-and-feasibility.md` | Updated | 53 |
| `docs/architect/status.md` | Updated | 94 |
| `docs/architect/system.md` | Updated | 76 |
| `infra/device-host/fixture_backend.py` | Created | 39 |
| `infra/device-host/install_tools.py` | Created | 85 |
| `infra/device-host/launch.example.json` | Created | 10 |
| `infra/device-host/mobile-qa-device.service` | Created | 22 |
| `infra/device-host/profile.toml` | Created | 16 |
| `infra/device-host/setup.sh` | Created | 11 |
| `infra/device-host/toolchain.lock.json` | Created | 125 |
| `justfile` | Updated | 61 |
| `scripts/test_ci_scope.py` | Created | 35 |

## Next steps

- Review the local implementation and create a PR when requested.
- Resolve host/model/spend inputs and prepare the concrete launch packet.
- After authorized real qualification, update canonical status and archive the plan.
