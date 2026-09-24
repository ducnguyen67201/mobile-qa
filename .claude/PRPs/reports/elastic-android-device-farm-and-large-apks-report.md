# Implementation report: Elastic Android hosts and large APKs

Date: 2026-09-22. Branch: `codex/elastic-android-device-farm`.

## Summary

Local implementation adds 2 GiB APK intake, resumable multipart uploads, durable
validation, authorized streamed delivery, a verified disk cache, supervisor-owned
Android slots, safe drain and power-control records, AWS deployment definitions,
and operator recovery and qualification tooling. Railway remains the application
hosting target; a fixed EC2 host supplies device capacity.

Cloud resources have not been provisioned. Paid image builds, actual device
qualification and hosted acceptance remain explicit release gates. The plan stays
active because those tasks have not been completed.

## Assessment versus plan

| Metric      | Plan                                                        | Actual                                                                            |
| ----------- | ----------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Complexity  | XL, multiple application and infrastructure boundaries      | XL; shared contracts, two migrations, browser/API/worker/controller/image changes |
| Files       | 60–80 estimated                                             | 156 current files, including generated consumers, plan and report                 |
| Capacity    | Candidate 1–2 slots, subject to qualification               | No approved or measured production slot count                                     |
| APK ceiling | 2 GiB inclusive                                             | 2,147,483,648 bytes in source contracts, storage and device intake                |
| Deployment  | Local implementation followed by authorized hosted campaign | Local implementation; no paid resources created                                   |
| Assurance   | Consolidated checks plus external release gates             | Local results below; hardware/provider behavior remains unverified                |

## Task status

| #   | Task                             | Status and evidence boundary                                                                                  |
| --- | -------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| 1   | Size compatibility and migration | Implemented; additive BIGINT/constraints, existing profile limits retained                                    |
| 2   | Multipart storage                | Implemented; signed bounded parts, receipts, completion/abort and sweeper; Railway interoperability gated     |
| 3   | Durable bounded validation       | Implemented; leased jobs, full streamed digest, scratch reservation and retry limits                          |
| 4   | Resumable browser upload         | Implemented; native part hashes, saved-part identity verification, progress/pause/resume                      |
| 5   | Worker delivery                  | Implemented; current lease authorization, remote signed URL or local stream                                   |
| 6   | APK cache                        | Implemented; app/digest namespace, cross-process locks, pinned readers, atomic publication and disk admission |
| 7   | Host/pool/slot records           | Implemented; scoped credentials, boot generations, qualification and app/profile bindings                     |
| 8   | Claim/readiness/accounting       | Implemented; reservations, fences, queue reasons and explicit preparation credit hold                         |
| 9   | Supervisor-owned cold slots      | Implemented for qualified generic direct execution; Minitap adapter limitation below                          |
| 10  | Drain/recovery                   | Implemented; fresh cleanup, intent/version fencing, uncertainty quarantine and operator resolution            |
| 11  | Images/startup                   | Docker/Packer/systemd source implemented; actual AMI bake and boot qualification gated                        |
| 12  | OpenTofu                         | Protected fixed host, encrypted retained EBS, scoped IAM, controller and schedule implemented; no apply       |
| 13  | Runtime capacity                 | Lambda plus durable wake outbox and scheduled reconciliation implemented; controller disabled by default      |
| 14  | Readiness/observability          | Customer queue states, trusted operator CLI and controller alarms implemented                                 |
| 15  | Warm policy                      | Local policy/evaluator implemented with warm target zero; enabling requires pristine reset qualification      |
| 16  | Heavy apps/density               | Campaign evaluation/report tooling implemented; no measurements or automatic slot approval                    |
| 17  | Checks/rollout/runbooks          | Implemented; validation record below                                                                          |
| 18  | Hosted pilot                     | Not executed; requires separate authorization and actual evidence                                             |

## Validation results

Generated clients were produced through `just types`. After the coherent source
batch, failed or invalidated checks were fixed and rerun without repeating unrelated
passing suites. API integration tests used fresh task-owned PostgreSQL databases
and cleaned them up; the existing local development/test databases were preserved.

| Check                               | Result                                                                                                                                                 |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Generated consumers and drift       | Pass; seven generated files updated, subsequent drift zero                                                                                             |
| Rust Clippy, all workspace targets  | Pass with warnings denied after final artifact fix                                                                                                     |
| API unit/integration                | All 80 current tests covered and passing across consolidated run and targeted reruns                                                                   |
| Pure Rust contracts                 | All 24 tests pass, including inclusive size limits                                                                                                     |
| Worker Ruff/Pyright                 | Pass; zero type errors                                                                                                                                 |
| Worker tests                        | All 243 current tests covered and passing across consolidated run and targeted reruns                                                                  |
| Browser formatting/typecheck/lint   | Pass; final rollout scope revalidated                                                                                                                  |
| Browser tests                       | Initial full suite: 189 pass; final changed API/orchestrator/page scope: 31 pass                                                                       |
| Browser production bundle           | Pass after final rollout fix; existing bundle-size warning remains                                                                                     |
| Capacity controller Ruff/Pyright    | Pass; zero type errors                                                                                                                                 |
| Capacity controller tests           | All 38 current tests covered and passing, including token scope and bootstrap failure boundaries                                                       |
| Deployment check helpers            | Three tests pass                                                                                                                                       |
| CI path selection                   | 36 cases pass                                                                                                                                          |
| OpenTofu                            | Format, credential-free init/validate and nested-virtualization provider schema gate pass                                                              |
| Packer and shell configuration      | Format, plugin/schema validation and shell syntax pass; no AMI build                                                                                   |
| Linux isolation launcher            | Strict C compilation passes in a disposable unprivileged Linux container; launcher not installed or executed                                           |
| Lambda image                        | Local amd64 build/import smoke passes; synthetic SDK import within 256 MiB memory limit                                                                |
| Native workspace build              | Pass with locked dependencies after the final artifact fix                                                                                             |
| API container                       | Local amd64 build passes; non-root UID 10001 smoke with networking disabled verifies CLI, Java 17, aapt2/apksigner, static assets and writable scratch |
| Hardware/provider/hosted acceptance | Not run; separately authorized release gates                                                                                                           |

New tests cover multipart identity/authorization, inclusive limits, recovery after
validation failure, grant rotation/boot fencing, claim-versus-drain races, uncertain
power outcomes, cache contention/pins, cancellation during preparation, warm policy,
private IPC and offline guest policy construction. Local fakes do not prove the
actual AWS/Railway/Android/kernel integrations.

Repository formatting and `git diff --check` also pass. Local image tags are
`mobile-qa-api:elastic-local` and `mobile-qa-capacity:elastic-local`; they were not
published. The runtime smoke did not start the API or call Doppler/AWS.

## Safety and operating defaults

- Multipart uploads are enabled automatically for remote object storage, without
  an environment toggle. Provider signing and browser CORS require hosted verification.
- The capacity controller is disabled in the default infrastructure inputs.
- Warm target is zero. Example CPU/memory allocations are hypotheses, not capacity promises.
- Stop requires current boot cleanup, no active reservations or finalization, and
  a final demand check. Unknown AWS outcomes are observed rather than blindly retried.
- Host/control/slot credentials have separate scopes. Runtime secrets are injected
  with Doppler; OpenTofu receives secret references only.
- EC2 creation starts billable compute. A stopped host retains billable EBS; API,
  database, storage and other services retain their independent charges.
- APK cache reuse avoids repeat external downloads on that host. Clean Android
  instances still install the APK. Cross-host reuse and cache eviction can require
  another download.

## Deviations and limitations

1. **Supervised adapter admission is explicit.** The existing standalone Minitap
   adapter is demo-specific and assumes its own emulator/backend. Supervised cloud
   pools currently admit qualified generic `android_direct_v1` execution contexts.
   The standalone demo path is preserved. A supervised Minitap adapter and its
   qualification remain additional work before claiming that capability.
2. **Qualification is external evidence.** Local fakes and source review cannot prove
   KVM, rendering, APK install time, density, pristine warm reset or packet isolation.
   The capacity report leaves `approved_slots=0` for explicit operator review.
3. **Network isolation is implemented and gated.** A root-owned cgroup/nftables
   policy and narrow fixed-path setuid launcher enforce the intended boundary.
   The exact kernel/emulator/image combination still needs a live packet campaign.
4. **Preparation changes the quoted maximum.** The hold includes up to 30 minutes
   of preparation plus cleanup allowance per possible attempt. Actual measured
   occupancy is settled under that cap; cold-start and warm idle are not new charges.
5. **The retained plan is not archived.** Its live qualification and hosted pilot
   criteria remain open, so an archived “complete” plan would overstate progress.

## Issues caught during implementation and validation

- Explicit unsigned schema constants fix the inclusive 2 GiB boundary.
- Grant revocation is checked inside the transaction after contention; old boot
  identities cannot mutate leases after revocation commits.
- Lost claim responses replay their existing lease while new admission is fenced.
- Drain includes older standalone reservations for newly bound physical devices;
  registration locks close the transition race and device identities are unique.
- A host with quarantined slots reports unserviceable capacity rather than waiting
  indefinitely as healthy paid compute.
- Finished attempts cannot obtain fresh APK capabilities.
- Cache publication and reservation changes share the cache index lock.
- Preparation rechecks cancellation after the last in-flight heartbeat completes.
- API image SDK placement permits access by its non-root runtime user.

## Review and release sequence

See [spec 11](../../../docs/architect/implementation/11-elastic-device-hosts-and-large-apks.md)
and [operations](../../../docs/architect/device-host-operations.md).

1. Review the local implementation and deployment inputs.
2. Authorize the priced cloud/image campaign before applying or building an AMI.
3. Prove Railway multipart/signing/CORS and large-APK behavior.
4. Qualify the exact host image, isolation, cleanup, restart and workload limits.
5. Run and record a complete upload/run/report/drain/stop/restart hosted pilot.
6. Enable automatic capacity only after its acceptance gates pass; raise slots or
   enable warm capacity only against measured evidence.

No commit, push, cloud deploy, image publish or production credential change was performed.

## Files changed

156 files in the current diff, including the retained plan, this report and generated consumers.

| File                                                                        | Action  | Lines        |
| --------------------------------------------------------------------------- | ------- | ------------ |
| `.claude/PRPs/plans/elastic-android-device-farm-and-large-apks.plan.md`     | Added   | +1099        |
| `.claude/PRPs/reports/elastic-android-device-farm-and-large-apks-report.md` | Added   | —            |
| `.dockerignore`                                                             | Added   | +22          |
| `.github/workflows/ci.yaml`                                                 | Updated | +48 / -0     |
| `Cargo.lock`                                                                | Updated | +12 / -0     |
| `Dockerfile.api`                                                            | Added   | +51          |
| `apps/api/Cargo.toml`                                                       | Updated | +2 / -0      |
| `apps/api/config/production.yaml`                                           | Updated | +3 / -3      |
| `apps/api/migration/src/lib.rs`                                             | Updated | +4 / -0      |
| `apps/api/migration/src/m20260922_000017_large_artifacts.rs`                | Added   | +48          |
| `apps/api/migration/src/m20260922_000018_device_hosts.rs`                   | Added   | +64          |
| `apps/api/src/app.rs`                                                       | Updated | +6 / -0      |
| `apps/api/src/config.rs`                                                    | Updated | +5 / -2      |
| `apps/api/src/controllers/device_hosts.rs`                                  | Added   | +100         |
| `apps/api/src/controllers/mod.rs`                                           | Updated | +2 / -0      |
| `apps/api/src/controllers/setup.rs`                                         | Updated | +78 / -3     |
| `apps/api/src/controllers/task_sessions.rs`                                 | Updated | +40 / -14    |
| `apps/api/src/controllers/worker.rs`                                        | Updated | +51 / -20    |
| `apps/api/src/services/apk_validation.rs`                                   | Updated | +19 / -7     |
| `apps/api/src/services/build_delivery.rs`                                   | Added   | +211         |
| `apps/api/src/services/capacity_control.rs`                                 | Added   | +435         |
| `apps/api/src/services/capacity_wake.rs`                                    | Added   | +135         |
| `apps/api/src/services/case_runs.rs`                                        | Updated | +1 / -0      |
| `apps/api/src/services/credit_billing.rs`                                   | Updated | +51 / -1     |
| `apps/api/src/services/device_hosts.rs`                                     | Added   | +663         |
| `apps/api/src/services/mod.rs`                                              | Updated | +8 / -0      |
| `apps/api/src/services/multipart_uploads.rs`                                | Added   | +405         |
| `apps/api/src/services/runs.rs`                                             | Updated | +6 / -1      |
| `apps/api/src/services/scheduler.rs`                                        | Updated | +8 / -0      |
| `apps/api/src/services/suite_runs.rs`                                       | Updated | +1 / -0      |
| `apps/api/src/services/task_sessions.rs`                                    | Updated | +20 / -5     |
| `apps/api/src/services/upload_validation.rs`                                | Added   | +301         |
| `apps/api/src/services/uploads.rs`                                          | Updated | +16 / -69    |
| `apps/api/src/services/worker_auth.rs`                                      | Updated | +9 / -1      |
| `apps/api/src/storage/mod.rs`                                               | Updated | +196 / -9    |
| `apps/api/src/storage/multipart.rs`                                         | Added   | +921         |
| `apps/api/src/tasks/execution.rs`                                           | Updated | +72 / -0     |
| `apps/api/tests/app_setup.rs`                                               | Updated | +1 / -1      |
| `apps/api/tests/device_hosts.rs`                                            | Added   | +929         |
| `apps/api/tests/execution.rs`                                               | Updated | +35 / -0     |
| `apps/api/tests/health.rs`                                                  | Updated | +2 / -0      |
| `apps/api/tests/large_artifacts.rs`                                         | Added   | +192         |
| `apps/mobile-worker/src/mobile_qa_worker/artifacts/__init__.py`             | Added   | +1           |
| `apps/mobile-worker/src/mobile_qa_worker/artifacts/cache.py`                | Added   | +283         |
| `apps/mobile-worker/src/mobile_qa_worker/artifacts/delivery.py`             | Added   | +86          |
| `apps/mobile-worker/src/mobile_qa_worker/artifacts/preparation.py`          | Added   | +69          |
| `apps/mobile-worker/src/mobile_qa_worker/automation/direct.py`              | Updated | +19 / -5     |
| `apps/mobile-worker/src/mobile_qa_worker/cli.py`                            | Updated | +29 / -2     |
| `apps/mobile-worker/src/mobile_qa_worker/device/android.py`                 | Updated | +67 / -11    |
| `apps/mobile-worker/src/mobile_qa_worker/execution/adapters.py`             | Updated | +6 / -0      |
| `apps/mobile-worker/src/mobile_qa_worker/execution/client.py`               | Updated | +91 / -0     |
| `apps/mobile-worker/src/mobile_qa_worker/execution/runner.py`               | Updated | +84 / -12    |
| `apps/mobile-worker/src/mobile_qa_worker/generated/models.py`               | Updated | +333 / -4    |
| `apps/mobile-worker/src/mobile_qa_worker/host/__init__.py`                  | Added   | +1           |
| `apps/mobile-worker/src/mobile_qa_worker/host/client.py`                    | Added   | +31          |
| `apps/mobile-worker/src/mobile_qa_worker/host/config.py`                    | Added   | +111         |
| `apps/mobile-worker/src/mobile_qa_worker/host/device.py`                    | Added   | +132         |
| `apps/mobile-worker/src/mobile_qa_worker/host/dispatch.py`                  | Added   | +61          |
| `apps/mobile-worker/src/mobile_qa_worker/host/ipc.py`                       | Added   | +146         |
| `apps/mobile-worker/src/mobile_qa_worker/host/isolation.py`                 | Added   | +78          |
| `apps/mobile-worker/src/mobile_qa_worker/host/policy.py`                    | Added   | +52          |
| `apps/mobile-worker/src/mobile_qa_worker/host/proxy.py`                     | Added   | +72          |
| `apps/mobile-worker/src/mobile_qa_worker/host/qualification.py`             | Added   | +125         |
| `apps/mobile-worker/src/mobile_qa_worker/host/runtime.py`                   | Added   | +147         |
| `apps/mobile-worker/src/mobile_qa_worker/host/slot.py`                      | Added   | +101         |
| `apps/mobile-worker/src/mobile_qa_worker/host/supervisor.py`                | Added   | +333         |
| `apps/mobile-worker/src/mobile_qa_worker/task_sessions.py`                  | Updated | +99 / -74    |
| `apps/mobile-worker/tests/test_artifact_cache.py`                           | Added   | +272         |
| `apps/mobile-worker/tests/test_artifact_delivery.py`                        | Added   | +299         |
| `apps/mobile-worker/tests/test_direct_automation.py`                        | Updated | +3 / -0      |
| `apps/mobile-worker/tests/test_execution.py`                                | Updated | +44 / -0     |
| `apps/mobile-worker/tests/test_guest_network.py`                            | Added   | +179         |
| `apps/mobile-worker/tests/test_host.py`                                     | Added   | +361         |
| `apps/mobile-worker/tests/test_task_sessions.py`                            | Updated | +78 / -0     |
| `apps/web/src/api/artifact-transfer.test.ts`                                | Added   | +67          |
| `apps/web/src/api/artifact-transfer.ts`                                     | Added   | +54          |
| `apps/web/src/api/generated/index.ts`                                       | Updated | +2 / -2      |
| `apps/web/src/api/generated/sdk.gen.ts`                                     | Updated | +223 / -2    |
| `apps/web/src/api/generated/types.gen.ts`                                   | Updated | +760 / -1    |
| `apps/web/src/api/generated/zod.gen.ts`                                     | Updated | +445 / -7    |
| `apps/web/src/api/multipart.test.ts`                                        | Added   | +81          |
| `apps/web/src/api/multipart.ts`                                             | Added   | +91          |
| `apps/web/src/api/setup.ts`                                                 | Updated | +2 / -1      |
| `apps/web/src/components/app/apk-upload.tsx`                                | Updated | +51 / -8     |
| `apps/web/src/hooks/use-apk-upload.ts`                                      | Updated | +120 / -20   |
| `apps/web/src/lib/multipart-upload.test.ts`                                 | Added   | +232         |
| `apps/web/src/lib/multipart-upload.ts`                                      | Added   | +157         |
| `apps/web/src/pages/AppDetail.test.tsx`                                     | Updated | +216 / -0    |
| `apps/web/src/pages/AppDetail.tsx`                                          | Updated | +6 / -1      |
| `apps/web/src/pages/RunDetail.test.tsx`                                     | Updated | +39 / -0     |
| `apps/web/src/pages/RunDetail.tsx`                                          | Updated | +7 / -0      |
| `apps/web/src/test/fixtures.ts`                                             | Updated | +1 / -0      |
| `contracts/browser.openapi.json`                                            | Updated | +2940 / -933 |
| `contracts/worker.schema.json`                                              | Updated | +988 / -39   |
| `crates/contracts/src/artifacts_api.rs`                                     | Added   | +201         |
| `crates/contracts/src/browser.rs`                                           | Updated | +10 / -7     |
| `crates/contracts/src/device_hosts.rs`                                      | Added   | +275         |
| `crates/contracts/src/device_hosts_api.rs`                                  | Added   | +106         |
| `crates/contracts/src/execution.rs`                                         | Updated | +11 / -6     |
| `crates/contracts/src/lib.rs`                                               | Updated | +5 / -0      |
| `crates/contracts/src/task_sessions.rs`                                     | Updated | +2 / -0      |
| `crates/contracts/src/worker.rs`                                            | Updated | +2 / -0      |
| `crates/contracts/src/worker/qualification.rs`                              | Updated | +2 / -2      |
| `crates/contracts/tests/execution.rs`                                       | Updated | +3 / -1      |
| `docs/architect/README.md`                                                  | Updated | +19 / -17    |
| `docs/architect/contracts.md`                                               | Updated | +18 / -1     |
| `docs/architect/decisions.md`                                               | Updated | +20 / -0     |
| `docs/architect/device-host-operations.md`                                  | Added   | +149         |
| `docs/architect/environment.md`                                             | Updated | +25 / -0     |
| `docs/architect/hosting.md`                                                 | Updated | +34 / -19    |
| `docs/architect/implementation/11-elastic-device-hosts-and-large-apks.md`   | Added   | +159         |
| `docs/architect/status.md`                                                  | Updated | +28 / -0     |
| `infra/aws/capacity-controller/.gitignore`                                  | Added   | +1           |
| `infra/aws/capacity-controller/Dockerfile`                                  | Added   | +14          |
| `infra/aws/capacity-controller/capacity_controller/__init__.py`             | Added   | +1           |
| `infra/aws/capacity-controller/capacity_controller/auth.py`                 | Added   | +61          |
| `infra/aws/capacity-controller/capacity_controller/aws.py`                  | Added   | +46          |
| `infra/aws/capacity-controller/capacity_controller/bootstrap.py`            | Added   | +85          |
| `infra/aws/capacity-controller/capacity_controller/control.py`              | Added   | +109         |
| `infra/aws/capacity-controller/capacity_controller/handler.py`              | Added   | +66          |
| `infra/aws/capacity-controller/capacity_controller/reconcile.py`            | Added   | +101         |
| `infra/aws/capacity-controller/pyproject.toml`                              | Added   | +27          |
| `infra/aws/capacity-controller/tests/test_auth.py`                          | Added   | +59          |
| `infra/aws/capacity-controller/tests/test_aws.py`                           | Added   | +50          |
| `infra/aws/capacity-controller/tests/test_bootstrap.py`                     | Added   | +66          |
| `infra/aws/capacity-controller/tests/test_control.py`                       | Added   | +54          |
| `infra/aws/capacity-controller/tests/test_handler.py`                       | Added   | +58          |
| `infra/aws/capacity-controller/tests/test_reconcile.py`                     | Added   | +196         |
| `infra/aws/capacity-controller/uv.lock`                                     | Added   | +369         |
| `infra/aws/device-pool/.gitignore`                                          | Added   | +7           |
| `infra/aws/device-pool/.terraform.lock.hcl`                                 | Added   | +39          |
| `infra/aws/device-pool/README.md`                                           | Added   | +139         |
| `infra/aws/device-pool/controller.tf`                                       | Added   | +93          |
| `infra/aws/device-pool/host.tf`                                             | Added   | +123         |
| `infra/aws/device-pool/iam.tf`                                              | Added   | +32          |
| `infra/aws/device-pool/outputs.tf`                                          | Added   | +4           |
| `infra/aws/device-pool/user-data.sh.tftpl`                                  | Added   | +13          |
| `infra/aws/device-pool/variables.tf`                                        | Added   | +101         |
| `infra/aws/device-pool/versions.tf`                                         | Added   | +19          |
| `infra/deployment/install_runtime.sh`                                       | Added   | +17          |
| `infra/device-host/ami/.gitignore`                                          | Added   | +2           |
| `infra/device-host/ami/README.md`                                           | Added   | +55          |
| `infra/device-host/ami/device-host.pkr.hcl`                                 | Added   | +58          |
| `infra/device-host/ami/provision.sh`                                        | Added   | +43          |
| `infra/device-host/ami/verify_image.py`                                     | Added   | +54          |
| `infra/device-host/mobile-qa-host.service`                                  | Added   | +52          |
| `infra/device-host/network/README.md`                                       | Added   | +72          |
| `infra/device-host/network/install.sh`                                      | Added   | +15          |
| `infra/device-host/network/launch-emulator.c`                               | Added   | +126         |
| `infra/device-host/network/mobile-qa-network.service`                       | Added   | +17          |
| `infra/device-host/network/policy.py`                                       | Added   | +179         |
| `justfile`                                                                  | Updated | +9 / -0      |
| `railway.toml`                                                              | Added   | +12          |
| `scripts/check_deployment.py`                                               | Added   | +175         |
| `scripts/test_check_deployment.py`                                          | Added   | +84          |
| `scripts/test_ci_scope.py`                                                  | Updated | +11 / -0     |
