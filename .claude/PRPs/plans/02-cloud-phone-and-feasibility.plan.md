# Plan: Phase 02 — qualify one cloud Android runner

## Summary

Extend the merged foundation with an operator-run qualification harness: one Linux/KVM host, one Android emulator, the pinned Minitap SDK, and one manually authored persistence test. Build a controlled demo APK, exercise good/broken/unavailable-prerequisite scenarios, and retain independent assertions, evidence, cleanup results and measured costs. The deliverable is a repeatable feasibility experiment, not the production job system.

## User Story

As the operator of our mobile QA service, I want to run a known test against an isolated Android emulator and independently verify its result, so that I can establish whether the runner is reliable enough to connect to customer app setup in phase 04.

## Problem → Solution

`fake` currently returns the outcome selected by a fixture; `sdk-import` only imports a package. Replace neither. Add an explicit real-device command that observes an installed app, distinguishes defects from unavailable prerequisites and ambiguous execution, and leaves evidence for every attempt.

## Metadata

- **Complexity:** XL, bounded into authoring, offline verification, and authorized cloud qualification.
- **Source specification:** `docs/architect/implementation/02-cloud-phone-and-feasibility.md`.
- **Phase:** 02; depends on the merged phase 01 foundation. Phase 01 manual browser acceptance remains open and is unrelated to this headless experiment.
- **Source baseline:** `main` at `d137e35`; inspected 2026-09-12 in `/Users/ducng/Desktop/workspace/mobile-qa`.
- **Scope:** approximately 50–55 files, including generated outputs, tests, a tiny Android fixture app and documentation; 12 ordered tasks. Most new app files are the isolated demo fixture, not product infrastructure.
- **Status:** tasks 1–11 authored and offline validation passed; task 12 live qualification pending. See [implementation report](../reports/02-cloud-phone-and-feasibility-report.md).
- **Authority:** `docs/architect/` owns product/architecture decisions. This PRP is the execution packet for that specification; it does not supersede it.
- **Confidence:** 8/10 for implementation against the inspected scaffold; actual device/model reliability is the experiment's output, not a forecast.

## Working rules and execution gates

1. Complete all agreed source, configuration, tests and documentation before running generation, formatting, typecheck, lint, build or tests. Task-level VALIDATE entries below describe the final verification phase, not checkpoints to run while editing.
2. Read-only source/toolchain research and explicit dependency preparation are permitted during authoring. Do not start an agent, emulator or model call to experiment against half-written code.
3. After authoring, run generation once and the affected offline checks. Batch any fixes and rerun only invalidated checks. No watchers, check hooks or full repository suite by default.
4. Produce a concrete cloud launch packet with current prices, exact region/SKU/AMI, access, expiry, storage and model budget before requesting paid-resource authorization. No resource creation or model spending is authorized by this planning request. Existing suitable host access can satisfy the host gate.
5. Code can be ready while live qualification is pending. Mark phase 02 complete only after the repeated real-device acceptance campaign and cleanup proof. A locally passing fake or mocked test is insufficient.
6. Phase 03 can proceed independently in API/auth/build/UI files. One integrator owns shared worker contracts, root recipes, CI filters and generated files. Do not launch extra agents simply because the roadmap permits concurrent work.

## UX design

Internal change — no customer dashboard transformation.

Before: operator runs `fake fixture.json` → predetermined JSON, no device.

After: operator prepares a host → runs `device-doctor` → runs `device-qualify` → receives `campaign.json` and `report.md` with links to per-attempt evidence → shuts down the host.

| Touchpoint           | Before                            | After                                                                                    | Failure behavior                                       |
| -------------------- | --------------------------------- | ---------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| `fake`, `sdk-import` | Implemented                       | Preserved                                                                                | Same JSON/exit conventions                             |
| `just device-smoke`  | Exits 2, explicitly unimplemented | One explicit controlled attempt, arguments required                                      | No hidden provisioning/install/model choice            |
| New `device-doctor`  | None                              | Read-only tooling/KVM/device profile report                                              | Nonzero, actionable reason, no model import            |
| New `device-qualify` | None                              | Nine ordered attempts plus interruption/recovery probes                                  | Keep all attempts; any mismatch makes campaign nonzero |
| Evidence             | No actual observations            | Private manifest, expected/observed comparison, PNG/XML, logs, SDK trace, optional video | Missing evidence has a reason; no invented green       |

## Mandatory reading and unified discovery

Line numbers refer to the inspected baseline. All paths are relative to the repository root unless marked SDK.

| Priority/category             | File:lines                                                                              | Pattern / reason                                                          |
| ----------------------------- | --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| P0 Authority                  | `AGENTS.md:1-30`, `docs/architect/README.md`                                            | Finish authoring before checks; canonical docs and app ownership          |
| P0 Scope                      | `docs/architect/implementation/02-cloud-phone-and-feasibility.md`                       | One Android emulator, independent evidence, reset, three repeats          |
| P0 Contracts                  | `docs/architect/contracts.md`, `crates/contracts/src/worker.rs:1-129`                   | Rust source; generated strict Pydantic; fixture-only types                |
| P0 Entry/error/logging        | `apps/mobile-worker/src/mobile_qa_worker/cli.py:19-60`                                  | Lazy SDK import, version guard, JSON stdout, sanitized logging, exit 2    |
| P0 Services/data flow         | `apps/mobile-worker/src/mobile_qa_worker/fake.py:12-47`                                 | Small functions; validate JSON on input and output                        |
| P0 Tests                      | `apps/mobile-worker/tests/test_fake.py:22-49,95-123`                                    | Parametrized boundaries, subprocess CLI, no SDK in fake mode              |
| P0 Configuration/dependencies | `apps/mobile-worker/pyproject.toml:1-39`, `apps/mobile-worker/uv.lock`                  | Python 3.12+, optional SDK 4.0.0; Ruff/Pyright strict; dotenv override    |
| P0 Generation                 | `crates/contracts/src/bin/export.rs:5-26`, `scripts/contracts.py:13-15,49-66`           | One worker schema root; staged generation; content-only writes            |
| P1 Ownership/lifecycle        | `scripts/runtime.py:68-103`                                                             | Private logs, owned process groups, bounded TERM/KILL cleanup             |
| P1 CLI recipe                 | `justfile`                                                                              | Explicit setup/check/dev; `device-smoke` is the intentional placeholder   |
| P1 CI                         | `.github/workflows/ci.yaml:21-83` and worker/contracts jobs                             | Whole-PR dependency filters; current `infra/**` unnecessarily selects API |
| P1 Configuration              | `docs/architect/environment.md`, `.gitignore`                                           | Doppler-only secrets, `.private/` excluded; no env files                  |
| P1 Status/next boundary       | `docs/architect/status.md`, `docs/architect/implementation/04-execution-and-reports.md` | No HTTP leasing yet; do not turn fixture protocol into scheduler          |
| P2 Import smoke               | `scripts/sdk_smoke.py:1-18`                                                             | Socket-blocked import only; preserve this low-cost check                  |
| P2 Documentation              | `docs/architect/commenting.md`, `docs/architect/dependencies.md`                        | Explain real/scaffold/experiment boundaries and upstream notices          |

**Eight-category finding:** naming uses snake_case functions/modules and PascalCase generated models; there is no worker repository/service class hierarchy or structured tracing subsystem to mirror. Use small modules/functions with dependency injection at the SDK and process boundaries. Logging today is standard `logging`; SDK logging differs and must be contained.

**Five traces:** (1) installed entry point → `cli.main`; (2) file JSON → generated validation → pure fake → validated JSON; (3) fake changes no external state, real harness will own only attempt directories and one designated emulator/backend; (4) Rust schema → existing exporter → Pydantic remains authoritative; (5) orchestration is a thin Python adapter, with Rust owning eventual product state. No new ORM/entity/controller is needed here.

## External research and pinned SDK findings

Read source without constructing Agent or initiating network/device work. SDK below means `apps/mobile-worker/.venv/lib/python3.12/site-packages/minitap/mobile_use/`. Reinstall from the existing lock if that path is absent; never edit site-packages.

Pinned distribution: `minitap-mobile-use==4.0.0`, wheel SHA-256 `65e6a8aa447d25257a6dbe99723f2ca5092d901ee447e78be44385144bb7b99e` in `uv.lock`. Current documentation can describe newer behavior; installed source is the integration authority.

| Topic                  | Primary source                                                                                                         | Finding                                                                                                                                                  |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SDK installation       | [Minitap installation](https://www.minitap.ai/docs/mobile-use-sdk/installation)                                        | Python 3.12+ and local ADB/device access; library import is not execution                                                                                |
| Agent lifecycle        | SDK `sdk/agent.py:92-245,531-574,763-794`                                                                              | `Agent(config=...)`, `await init()`, `await run_task(request=...)`; `stop_current_task()` requests cancellation                                          |
| Configuration          | SDK `sdk/types/agent.py:94-125`; `sdk/builders/agent_config_builder.py:62-72,99-128,223-264,359-416`                   | Explicit device/platform/profile; callbacks; builder validation can touch provider credentials                                                           |
| Task limits and traces | SDK `sdk/types/task.py:63-115`; `sdk/agent.py:1118-1160`                                                               | Max steps, trace paths and unique task name; traces finalize after execution and may fail independently                                                  |
| Result semantics       | SDK `sdk/agent.py:693-717`; `sdk/types/task.py:177-207`                                                                | `run_task` returns content, not a promised TaskResult object; completed status reflects execution, not business truth                                    |
| Environment/logging    | SDK `config.py:15-45,131-198`; `utils/logger.py:50-71`                                                                 | SDK reads dotenv/Pydantic env file and writes logs relative to cwd; stdout contains SDK log text                                                         |
| Usage                  | SDK builder `with_graph_config_callbacks`; installed `langchain_core/callbacks/base.py:655-665`                        | Callback can observe LLM completion; normalized usage requires guarded parsing, deduplication and explicit missing values                                |
| Cloud KVM              | [AWS nested virtualization](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/amazon-ec2-nested-virtualization.html) | M8i is supported; launch explicitly enables nested virtualization. Verify actual region/type before purchase                                             |
| Android acceleration   | [Android acceleration](https://developer.android.com/studio/run/emulator-acceleration)                                 | Linux requires usable KVM and permissions; a CPU flag alone is insufficient                                                                              |
| Emulator lifecycle     | [Android command line](https://developer.android.com/studio/run/emulator-commandline)                                  | Docs announce `android emulator` transition; pin the tool invocation. `-no-snapshot` prevents snapshot load/save; wipe does not clear a separate SD card |
| Evidence/control       | [ADB](https://developer.android.com/tools/adb)                                                                         | Explicit serial, `exec-out screencap -p`, bounded screenrecord; video has duration/rotation limits                                                       |
| Demo build             | [AGP 8.13 compatibility](https://developer.android.com/build/releases/agp-8-13-0-release-notes)                        | Pin AGP 8.13.0, Gradle 8.13, JDK 17, Build Tools 35.0.0; compile/target API 35                                                                           |

### KEY_INSIGHT / APPLIES_TO / GOTCHA

- **KEY_INSIGHT:** The SDK can silently select the first device if either ID or platform is missing. **APPLIES_TO:** preflight and adapter. **GOTCHA:** always set both `device_id` and `DevicePlatform.ANDROID`; verify the serial and reject unowned devices.
- **KEY_INSIGHT:** The SDK names completed traces with `_PASS`. **APPLIES_TO:** evidence and classification. **GOTCHA:** retain trace as raw execution evidence; never derive our verdict from its filename, log text or returned content.
- **KEY_INSIGHT:** `stop_current_task()` is cooperative and synchronous Android calls may outlive cancellation. **APPLIES_TO:** supervisor. **GOTCHA:** a separate process group plus host cgroup/cleanup verification is required; do not reuse capacity merely because a coroutine stopped.
- **KEY_INSIGHT:** Setting `PYTHON_DOTENV_DISABLED=1` blocks python-dotenv discovery, but SDK Pydantic Settings independently names `.env`. **APPLIES_TO:** SDK subprocess. **GOTCHA:** use a newly created private cwd with no `.env`, set the flag and telemetry false before import, and provide an allowlisted process environment. Do not patch the SDK or rely solely on the flag.
- **KEY_INSIGHT:** SDK local video tools require extra model/ffmpeg configuration. **APPLIES_TO:** artifacts. **GOTCHA:** do not enable them for passive recording; own ADB screenrecord separately and label gaps/unavailability.
- **KEY_INSIGHT:** Source exposes no general public local-device `close()` method in the inspected Agent. **APPLIES_TO:** lifecycle. **GOTCHA:** do not invent `async with Agent`, `agent.close`, streamed event APIs or private-task access.

## Strategic design

### Approach and alternatives

Keep Minitap responsible for UI navigation, with our deterministic verifier and lifecycle around it. Use a real cloud Linux emulator, not a second agent SDK controlling the same taps. Build a tiny Java/Android Views fixture app so known-good/broken builds share the same UI and source; Java avoids adding Kotlin/Compose complexity to this experiment. Use the normal pinned Gradle Android build rather than maintaining a custom APK packager.

Managed device farms remain a fallback if KVM or reset cannot qualify. Do not implement provider abstraction layers, Terraform modules or Kubernetes now. A concrete launch JSON/runbook plus idempotent host setup and one host service is enough.

### Ownership and proposed file map

| File or tightly scoped group                                                                                                                                           | Action        | Purpose                                                                                                                         |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `crates/contracts/src/worker/qualification.rs`                                                                                                                         | CREATE        | Namespaced qualification request/result/evidence wire types                                                                     |
| `crates/contracts/src/worker.rs`                                                                                                                                       | UPDATE        | Declare qualification module and add its export root; keep fake types unchanged                                                 |
| `crates/contracts/tests/qualification.rs`                                                                                                                              | CREATE        | Version/range/result invariant tests                                                                                            |
| `contracts/worker.schema.json`, `apps/mobile-worker/src/mobile_qa_worker/generated/models.py`                                                                          | GENERATE      | Existing pipeline only                                                                                                          |
| `apps/mobile-worker/src/mobile_qa_worker/cli.py`                                                                                                                       | UPDATE        | Explicit doctor/run/campaign/internal-child commands; preserve fake/import                                                      |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/{__init__,config,device,runner,sdk_adapter,evidence,verifier,campaign}.py`                                      | CREATE        | Small operator harness modules; no generic plugin framework                                                                     |
| `apps/mobile-worker/tests/test_{qualification,device,verifier,sdk_adapter,campaign}.py`                                                                                | CREATE        | Offline behavior, process and contract coverage                                                                                 |
| `apps/mobile-worker/tests/fixtures/qualification/`                                                                                                                     | CREATE        | Minimal XML and SDK callback payload fixtures, no binaries/secrets                                                              |
| `apps/mobile-worker/pyproject.toml`, `apps/mobile-worker/uv.lock`                                                                                                      | UPDATE        | Declare direct callback dependency in sdk extra at existing locked version; add coverage markers if needed                      |
| `apps/qa-demo-android/{settings.gradle,build.gradle,gradle.properties,gradlew,gradlew.bat,gradle/wrapper/gradle-wrapper.properties,gradle/wrapper/gradle-wrapper.jar}` | CREATE        | Isolated fixture-only Android project/wrapper; no root build coupling                                                           |
| `apps/qa-demo-android/app/build.gradle`, `app/src/main/AndroidManifest.xml`, `app/src/main/java/ai/mobileqa/demo/MainActivity.java`, `app/src/main/res/values/ids.xml` | CREATE        | One screen, persistence variants, stable accessible IDs                                                                         |
| `infra/device-host/{toolchain.lock.json,profile.toml,setup.sh,mobile-qa-device.service,launch.example.json}`                                                           | CREATE        | Nonsecret toolchain/profile, idempotent bootstrap, designated service, reviewable launch example                                |
| `infra/device-host/fixture_backend.py`                                                                                                                                 | CREATE        | Loopback-only controlled session endpoint; no product backend                                                                   |
| `justfile`, `.gitignore`, `.github/workflows/ci.yaml`                                                                                                                  | UPDATE        | Explicit device commands, fixture outputs ignored, affected CI routing                                                          |
| `scripts/test_ci_scope.py`                                                                                                                                             | CREATE        | Small explicit path-selection regression tests against the actual filter config                                                 |
| `docs/architect/{device-qualification.md,environment.md,development.md,status.md,dependencies.md,README.md}`                                                           | CREATE/UPDATE | Operating runbook, secrets, evidence and provenance                                                                             |
| `AGENTS.md`                                                                                                                                                            | UPDATE        | Clarify old “no device/model in this phase” constraint was phase 01; qualification is explicit, fake/check paths remain offline |

An added file is justified by ownership or meaningful test boundaries, not a required file quota. Keep root manifests and web/API runtime unchanged. `scripts/contracts.py` already walks schema definitions through WorkerContracts and should need no change.

### NOT building

- Customer auth/upload UI (phase 03), HTTP claims/heartbeats/leases, Postgres run records, retries or production report endpoints (phase 04).
- Case/suite/plan CRUD, AI test generation, general-purpose assertion DSL, self-healing tests.
- iOS, physical devices, arbitrary customer APK qualification, multiple devices or tenants per host.
- Device pool autoscaling, managed farm adapters, streaming dashboard, production S3 pipeline.
- Automatic cloud creation on setup/test/run, new agent framework, global editor/SDK configuration.

### Controlled application and independent assertion

Package/activity: `ai.mobileqa.demo/.MainActivity`; fixed case ID `persist-task-v1`. Two product flavors `good` and `broken`, same package and layout. Good stores tasks durably in SharedPreferences using a successful synchronous commit before displaying the saved row. Broken displays the identical row in memory and loses it on process death. Native-free APKs work on x86_64; validate absence of native libraries rather than requiring a nonexistent ABI declaration.

Use accessible resource IDs `task_input`, `save_task`, `task_list`, `ready_marker`, `prerequisite_unavailable`; task row text equals the unique task value. Disable Android auto-backup for the demo. Use ordinary platform Views and an executor for HTTP work, not networking on the main thread.

A tiny stdlib HTTP server on host loopback port 8765 answers `/session` with 200 or 503. The demo checks `http://10.0.2.2:8765/session` (emulator host alias), with bounded connect/read timeout. Configure cleartext only for this controlled fixture host. The ready screen appears only after 200; 503 renders the explicit unavailable marker. No real login credentials or remote data are needed. The campaign owns backend mode and resets its process between attempts. Runner input never contains an expected verdict.

Each attempt uses `qa-<attempt UUID>` as exact ASCII task text:

1. Confirm installed APK checksum/package, backend reachability and foreground ready/unavailable marker. Capture initial PNG/XML. Record prerequisite HTTP status from both host fixture probe and the app UI; host reachability alone does not prove emulator network readiness.
2. For a ready app, instruct Minitap to enter the task and save it, then stop on the task list. Do not disclose flavor or expected result to the agent.
3. Independently dump UI hierarchy; require ready marker and exact task row in our app package before continuing. Without proof of creation, result is inconclusive, never a persistence failure.
4. After SDK execution is stopped, force-stop the app and relaunch the explicit activity via ADB. Wait for a fresh complete hierarchy. Do not clear data between create and reopen.
5. Require ready marker and a readable hierarchy. Exact row present → passed; absent after bounded stable observations → failed. A backend-error screen, empty/corrupt dump, wrong foreground package or screenshot failure → blocked/inconclusive as defined below.
6. Keep before-create, after-create and after-reopen evidence. Then reset by removing the owned writable AVD state and booting fresh. Assert prior task and session state are absent after reinstall/launch; cleanup verification does not change a recorded business observation but prevents reuse and campaign success.

This demonstrates one deterministic persistence assertion. It does not claim a generic verifier for arbitrary apps or remote account reset.

### Result contract and classification

Add `pub mod qualification;` under worker.rs, with `QualificationContracts` reachable as an additional field of the schema-only WorkerContracts root. Existing request/result/probe shapes and fixtures stay unchanged. New type names have Qualification prefixes to avoid codegen collisions. Use Serde `deny_unknown_fields`, snake_case enum values, UUIDs, UTC timestamps, explicit version validation and constrained nonnegative values. Schema annotations alone are not Rust validation.

**QualificationRequest:** version=1, attempt_id UUID, case_id=`persist-task-v1`, apk_path (operator-local), expected_apk_sha256 (64 lowercase hex), package=`ai.mobileqa.demo`, activity, serial, profile_path, output_root. The CLI constructs and strictly validates it; no expected-outcome field. Validate canonical existing APK/profile paths and artifact containment; never accept a shell command or URL from this object.

**QualificationResult:** version, attempt_id, case_id, outcome (`passed|failed|blocked|inconclusive`), reason_code, expected_behavior, observed_behavior, started_at/ended_at UTC, requested build/package/profile identity plus independently observed build_sha256/package/device inventory, model profile hash and model identifiers, reset status (`verified_clean|quarantined|not_started`), phase durations in milliseconds, artifacts, usage and cost coverage. Observed metadata can be absent with a reason when a preflight fails; never copy requested metadata into an observed field. Timestamps must be ordered; every available artifact includes relative path, MIME type, byte length and SHA-256. Missing artifacts contain `status=unavailable` plus reason rather than a fake path. No credentials or arbitrary exception text.

Use integer counts with explicit safe bounds or canonical decimal strings for potentially large token/cost counters; use integer micro-USD or Decimal-derived strings for estimates, never floating-point money. Optional usage/cost fields distinguish unavailable from measured zero. Worker-local process handles/settings use dataclasses, not duplicate transport models. Campaign oracle/aggregation remains Python-local in phase 02 and is explicitly experimental.

| Condition                                                                                | Outcome / reason                             | Required distinction                                    |
| ---------------------------------------------------------------------------------------- | -------------------------------------------- | ------------------------------------------------------- |
| Created task proven and survives verified reopen                                         | passed / persistence_observed                | All required evidence present                           |
| Created task proven, ready app reopens, exact task is absent                             | failed / persistence_lost                    | A real violated expectation, independent of SDK claim   |
| Backend 503/login prerequisite unavailable                                               | blocked / prerequisite_unavailable           | Capture actual unavailable screen; no model call needed |
| APK unsupported/invalid, KVM unavailable, install fails before test                      | blocked / device_or_build_unavailable        | Setup category, not an app defect                       |
| SDK timeout/error, step budget exhausted, missing decisive evidence, creation not proven | inconclusive / specific reason               | Never convert uncertainty to a failed assertion or pass |
| Cancellation during execution                                                            | inconclusive / cancelled                     | Preserve partial artifacts; no automatic replay         |
| Cleanup fails after observed verdict                                                     | Preserve observed outcome, reset=quarantined | Campaign fails and device cannot be reused              |

A single-run CLI returns 0 when it durably writes a valid terminal result, even for failed/blocked/inconclusive; invalid invocation or inability to persist a result exits 2. Campaign returns 0 only for all expected classifications plus recovery/cleanup acceptance, 1 for qualification mismatch/quarantine, 2 for invalid setup. Document both conventions. Do not parse noisy SDK stdout as result JSON; use a private child-result file, validated by the parent.

### Lifecycle, evidence and bounds

- One designated emulator serial (initially emulator-5554), one attempt at a time, one service user. Acquire a host-local advisory `flock` before touching device/backend; the lock is not a production worker lease.
- Separate supervisor, emulator and SDK child. A systemd service contains the supervisor's full process tree with `KillMode=control-group`, `Restart=no`, bounded runtime/stop timeout. Local subprocesses use `start_new_session=True`; retain ownership records and terminate all owned groups even if the immediate parent has exited. Never `pkill`, kill unrelated ADB servers, or wipe an arbitrary user AVD.
- The supervisor exclusively owns boot/stop/reset; the host service runs that supervisor, not a competing emulator manager. Before mutation write a durable dirty/quarantine marker in the owned state root. On interruption or stale marker, refuse reuse until explicit recovery kills remaining owned processes, discards owned writable state and verifies a clean boot. PID alone is insufficient ownership evidence; include boot identity/start information and service/attempt identity.
- Suggested initial limits in profile: boot 180s, ADB command 20s, install 90s, SDK init 60s, navigation 180s/max_steps 30, each assertion wait 15s, cleanup 90s. Enforce an overall attempt budget plus a separate cleanup grace; measure and revise deliberately. Max steps is graph recursion, not a promised count of taps or model requests.
- APK ≤100 MiB for this experiment; evidence ≤250 MiB/attempt, bounded log files; minimum host disk free preflight. Bound XML size/depth and use safe parsing; invalid/corrupt dumps produce inconclusive. No customer input is evaluated as code or shell syntax.
- Use private 0700 attempt directories and 0600 files. Atomically persist result (`temp write`, flush/fsync, rename); finalized attempts are immutable. Keep all retries as new attempts. Put `TMPDIR` inside the attempt so SDK trace temp files remain attributable. SDK logs go to the private child cwd, not root source paths.
- Capture controller event JSONL with safe phase/attempt/reason identifiers, raw SDK log as private diagnostic, screenshots/XML as primary assertion evidence, bounded device logcat, SDK trace output, optional bounded ADB video. Do not publish raw model reasoning/logs as the customer report. Treat raw demo traces as private and never include secret values in prompts.
- Passive video may stop at 180s or fail on a headless renderer: record coverage/gaps and an unavailable reason. PNG/XML evidence is required for this assertion; video is supplementary. Trace filename `_PASS` carries no application verdict.
- Phase 02 stores evidence on encrypted host disk and explicitly copies bundles to the operator before termination; measure transfer duration/checksum. No production object-store API. Copy failure leaves evidence on the stopped host until recovery; do not silently terminate the only copy.

### SDK adapter exact integration seam

All Minitap imports occur inside the dedicated child, after environment preparation. Imports: `Agent` from `minitap.mobile_use.sdk`; `AgentConfigBuilder` from `.sdk.builders.agent_config_builder`; `AgentProfile`, `TaskRequest`, `DevicePlatform` from `.sdk.types`; `LLMConfig` from `minitap.mobile_use.config`.

Build an explicit nonsecret profile with all five agent nodes and utils.outputter/hopper, including each fallback. For the first experiment use only OpenAI provider with model ID supplied in the operator profile; no automatic SDK default/multi-provider fallback or unverified model name. Require `OPENAI_API_KEY` at runtime from Doppler. Construct the LLMConfig explicitly from validated profile data; setting the same chosen model as fallback avoids hidden provider selection. Actual compatibility/cost is qualified in the campaign.

`AgentConfigBuilder().for_device(platform=DevicePlatform.ANDROID, device_id=serial).add_profile(profile).with_default_profile(profile.name).with_graph_config_callbacks([usage_callback]).build()` → `Agent(config=config)` → bounded `await agent.init()` → `await agent.run_task(request=TaskRequest(goal=goal, task_name=str(attempt_id), max_steps=limit, record_trace=True, trace_path=private_trace_path, locked_app_package=package))`. Install via our ADB preflight first, so do not also set `app_path` on the task.

Do not rely on untyped SDK output. Use a small typed SDK port/protocol for our runner, with the concrete implementation and any justified narrow typing accommodation confined to sdk_adapter.py. Do not disable strict mode globally. Do not use SDK private `_tasks` or infer a public event hook from Task.on_status_changed.

Declare `langchain-core==1.6.3` in the sdk extra if importing `AsyncCallbackHandler` directly (already locked transitively). Implement on_llm_end with the installed signature; normalize response usage metadata defensively, deduplicate by callback run_id, retain model identity and mark unknown calls. Never log prompts or completions from callbacks. Callback parsing failure must not break execution. Parent records monotonic elapsed durations; usage unavailable is not zero and cost is marked partial/unavailable. Budget limits stop new attempts, while acknowledging that in-flight provider calls may have already incurred cost.

Doppler wraps only the SDK child command, not the emulator/backend or offline checks. In `_sdk-run`, capture the required provider key and rebuild the SDK process environment from approved runtime keys (PATH, locale, required Python runtime paths, TMPDIR, model credential), force telemetry=false and dotenv disabled, then import SDK. Do not pass Doppler token, unrelated server secrets or inherited tracing exports onward. Use a fresh private cwd with no env file. Child stdin is closed to prevent credential/telemetry prompts. Do not configure global Doppler auth during implementation.

## Patterns to mirror (actual source examples)

### Naming and boundary validation

Source: `apps/mobile-worker/src/mobile_qa_worker/fake.py:12-17`:

```python
def parse_request(raw: str) -> FakeExecutionRequest:
    """Validate fixture JSON without coercing values such as string versions to integers."""
    request = FakeExecutionRequest.model_validate_json(raw, strict=True)
    if request.version != 1:
        raise ValueError("unsupported fixture protocol version")
    return request
```

### Error handling and logging

Source: `apps/mobile-worker/src/mobile_qa_worker/cli.py:54-60`:

```python
except (ValueError, OSError, ImportError, importlib.metadata.PackageNotFoundError) as exc:
    # Avoid dumping fixture payloads, paths or credentials into logs.
    logging.error(
        "Input or setup failed (%s). Check the fixture or installed dependency.",
        type(exc).__name__,
    )
    return 2
```

Mirror safe logging; add qualification-specific reason codes rather than copying every upstream exception string.

### Process ownership

Source: `scripts/runtime.py:95-101`:

```python
if child.poll() is None:
    os.killpg(child.pid, signal.SIGTERM)
    try:
        child.wait(timeout=10)
    except subprocess.TimeoutExpired:
        os.killpg(child.pid, signal.SIGKILL)
        child.wait()
```

Adapt, do not copy blindly: the new harness must clean surviving descendants even if the immediate child already exited. The existing development supervisor is not a complete hostile-process sandbox.

### Test structure

Source: `apps/mobile-worker/tests/test_fake.py:22-30`:

```python
@pytest.mark.parametrize(
    ("kind", "outcome"), [("pass", "passed"), ("fail", "failed"), ("blocked", "blocked")]
)
def test_fake_is_deterministic(kind, outcome):
    request = parse_request((FIXTURES / f"{kind}.json").read_text())
    first = execute(request).model_dump(mode="json")
    assert first == execute(request).model_dump(mode="json")
    assert first["outcome"] == outcome
    assert first["run_id"] == str(request.run_id)
```

Repository pattern: not applicable; no DB in this phase. Service pattern: small typed functions and injected SDK/process interfaces, matching fake.py and runtime.py; no new repository abstraction.

## Step-by-step tasks

### Task 1 — Freeze scope and host/toolchain inputs

- **ACTION:** Update phase ownership comments; author profile, toolchain manifest, bootstrap/runbook and launch packet format.
- **IMPLEMENT:** Candidate Linux Ubuntu 24.04 x86_64 on M8i 4-vCPU/16-GiB sizing; Android API35 Google APIs x86_64, portrait 1080×1920/420dpi/en-US/UTC. Resolve actual downloadable emulator/platform-tools/command-line-tools/system-image revisions from official Android repository metadata during dependency preparation; store exact URLs, checksums and installed revisions, not `latest`. Use the existing emulator binary interface only when the pinned release supports its documented flags; record that the newer CLI transition is not yet qualified. Provisioning inputs remain explicit region/AMI/network/identity/expiry/spend fields, not invented IDs.
- **MIRROR:** environment.md and explicit setup.py install separation.
- **IMPORTS:** setup shell uses bounded commands; Python config uses pathlib/tomllib/dataclasses.
- **GOTCHA:** API35 image path alone is not an immutable revision; Apple Silicon local execution is not proof of Linux x86_64 cloud qualification. Host license acceptance is explicit.
- **VALIDATE:** At final verification, checksum/version mismatch and missing KVM/profile fail before model import; no setup command creates paid resources. Reject unresolved lock placeholders before declaring code ready.

### Task 2 — Define qualification wire contracts

- **ACTION:** Add qualification.rs, root reachability and Rust contract tests.
- **IMPLEMENT:** Request/result/evidence/usage structures above, semantic validators, version and path/hash bounds. Preserve fake enum/model semantics. Add matching strict JSON fixtures under worker test fixtures; no HTTP job identifiers or lease fields.
- **MIRROR:** worker.rs Serde/Schemars patterns; schema export root.
- **IMPORTS:** chrono, schemars, serde, uuid already in pure contracts crate.
- **GOTCHA:** Do not use public doc comments to promise constraints that Rust never enforces; validate negative/overflow/nonfinite/missing/extra values consistently. Python-local config does not become a second public contract.
- **VALIDATE:** After all tasks are authored, run existing generation once; cross-language version/enum/hash/time/result invariants; fake contract fixtures unchanged; browser generated output has no diff.

### Task 3 — Author the demo APK and controlled backend

- **ACTION:** Add the fixture Android app and loopback backend.
- **IMPLEMENT:** Good/broken persistence variants, stable IDs, fixture session gate, no auto-backup; single Java activity and minimal resources. Pin Gradle wrapper with distribution SHA; AGP8.13.0/Gradle8.13/JDK17/BuildTools35.0.0, compile/target35, min26. Flavor controls persistence implementation only, not UI or package. Backend has explicit `--mode ready|unavailable`, loopback binding and no hidden mutation endpoint. Wrapper/build caches and APKs ignored; record build hashes rather than commit APKs.
- **MIRROR:** apps ownership; stdlib CLI handling.
- **IMPORTS:** Android Activity/View/SharedPreferences/HttpURLConnection; backend http.server/argparse.
- **GOTCHA:** Check successful durable save before presenting a row; do not make the verifier depend on an exposed “expected outcome” or flavor label.
- **VALIDATE:** Final Gradle assemble/lint; final real-device campaign proves both variants and backend503. Inspect APK manifest and native-library metadata. No new demo framework test boilerplate required.

### Task 4 — Implement profile, ADB preflight and reset

- **ACTION:** Add config.py and device.py with injected command runner.
- **IMPLEMENT:** Validate owned state root, Linux/KVM, pinned tools, disk, serial, ADB state, boot property, package manager and PNG; parse APK package/minSdk/native libraries/checksum; install and launch; configure/read back device profile; fresh attempt AVD writable directory with no reusable SD card/snapshots. Recovery discards only owned state after stopping owned processes. Backend resets independently.
- **MIRROR:** subprocess ownership and frozen configuration.
- **IMPORTS:** subprocess, pathlib, hashlib, zipfile, time, dataclasses, contextlib; safe XML parsing for bounded hierarchy.
- **GOTCHA:** Never select first device, use `adb -s serial` for every device operation; no global adb kill-server. APK without native libraries is valid. Boot-completed alone is insufficient.
- **VALIDATE:** Offline command transcripts cover offline/unauthorized/wrongserial, incompatible APK, corrupt PNG, timeout, permission/disk failure and unrelated-device protection. Live doctor proves exact toolchain/profile.

### Task 5 — Implement SDK process boundary and usage

- **ACTION:** Add sdk_adapter.py and `_sdk-run` command, declare callback dependency without upgrading unrelated pins.
- **IMPLEMENT:** Exact integration described above; lazy imports, allowlisted environment, telemetry disabled, clean cwd/TMPDIR, explicit noninteractive model profile, graph callbacks, private child-result file, typed SDK port and process termination protocol. SDK output is diagnostic only.
- **MIRROR:** cli.sdk_import and frozen sdk extra.
- **IMPORTS:** Minitap public imports listed above; langchain_core.callbacks.AsyncCallbackHandler; importlib.metadata, asyncio, logging, os.
- **GOTCHA:** SDK stdout logs contaminate JSON; no invented close/stream methods; importing Settings before environment preparation freezes wrong settings. Awaited timeout alone cannot guarantee process death.
- **VALIDATE:** Mock SDK configuration call sequence, no constructor/import in fake/doctor; contaminated stdout, absent credentials, callback duplicates/missing usage, disabled dotenv/telemetry and no SDK file writes outside private attempt root.

### Task 6 — Implement assertion and evidence writer

- **ACTION:** Add verifier.py and evidence.py.
- **IMPLEMENT:** Classify only from package-scoped stable UI evidence before and after deterministic reopen. Record exact task text, hierarchy/PNG checksums and missing reasons; safe relative artifact paths; atomic final JSON and readable Markdown. Trace _PASS and agent output never participate in business classification. Track monotonic durations and explicit cost coverage.
- **MIRROR:** strict generated input/output; private output directory conventions.
- **IMPORTS:** generated Qualification models, json, hashlib, pathlib, datetime.UTC, decimal, XML parser with bounded input.
- **GOTCHA:** Absent after reopen is failure only if creation was proven and ready screen is conclusively observed. Do not clear app data until after the assertion. Escape user-visible report text and paths; no shell/HTML execution.
- **VALIDATE:** Decision table below, path traversal/symlink refusal, malformed hierarchy, missing PNG, timestamp/hash checks, disk-full/partial-write behavior and reports that remain readable without the SDK installed.

### Task 7 — Implement supervisor and CLI

- **ACTION:** Add runner.py and public device-doctor/device-run command dispatch.
- **IMPLEMENT:** Validate request → acquire host lock → dirty marker → clean boot/install → backend/initial evidence → SDK navigation → independent assertion → evidence persistence → verified reset or quarantine → final result. Launch SDK through Doppler only for actual navigation; close stdin; owned process groups and bounded cleanup. Host systemd unit runs the supervisor as the device user with KVM access, no automatic restart and cgroup cleanup.
- **MIRROR:** runtime.py lifecycle, CLI safe errors and JSON exit semantics.
- **IMPORTS:** fcntl (Linux/macOS), signal, subprocess, uuid, contextlib; qualification modules.
- **GOTCHA:** Lock release is not proof of clean device state. Interrupted dirty marker survives parent death; child-exited/grandchild-alive scenario must be handled. Final result must report cleanup quarantine even after a successful assertion.
- **VALIDATE:** Offline subprocess probes for TERM/KILL escalation, surviving child group, second invocation busy, stale marker recovery, no unrelated process termination, no external calls in unit tests.

### Task 8 — Implement campaign and measurement

- **ACTION:** Add campaign.py plus explicit device-qualify command.
- **IMPLEMENT:** Cycle good/ready, broken/ready, good/unavailable three times (nine distinct attempts) with fresh reset each time. Keep expected classification in campaign oracle only. Add interruption during navigation and a post-recovery good attempt. Include contamination sentinel probe proving recovery clears task/session data. Keep attempts immutable and retain mismatches, trace gaps, usage coverage, transfer duration and cost estimate with rate source/date. Campaign stops on quarantine or exceeded budget.
- **MIRROR:** parametrized scenario tests, exact-run identifiers.
- **IMPORTS:** runner/evidence modules, uuid, time, decimal, pathlib.
- **GOTCHA:** Nine runs is an engineering gate, not statistical reliability or production coverage. Do not rerun until green and omit failures. Model/host price absent means unavailable estimate, not free execution.
- **VALIDATE:** Offline oracle does not leak into child request; campaign exit1 for any mismatch/cleanup failure, missing attempt or missing decisive evidence. Live nine-run classification and recovery gates mandatory.

### Task 9 — Wire explicit commands and affected CI

- **ACTION:** Replace device-smoke stub, add recipes and narrow device checks.
- **IMPLEMENT:** Public CLI surface specified below; no device command is a dependency of setup/check/build/smoke. Split API `infra/**` into `infra/compose.yaml` and `infra/postgres/**`; map `infra/device-host/**` into qualification/offline-worker checks. Add `crates/contracts/src/worker/**` to worker filter. Add a demo-build job only for demo/build-toolchain inputs, using JDK17 and pinned Android tools, no emulator or model. Keep whole-PR comparison and cancellation. The current worker CI installs only base/dev dependencies: update its explicit sync to include `--extra sdk` so strict Pyright resolves the new SDK/callback imports; cache this environment. Installing SDK is not permission to import it during fake tests or run it. Add path regression tests reading actual filter YAML (use existing pinned PyYAML in worker dev environment) and select them when CI/filter fixtures change. Shared workflow edits will trigger the existing broad jobs for this PR; normal later device-only edits must not.
- **MIRROR:** current dorny path filters and explicit justfile commands.
- **IMPORTS:** test_ci_scope uses pathlib/yaml and the actual filter matcher semantics; reuse pinned picomatch through web's existing installed tooling for matching rather than approximate glob semantics. Run this Node/Python scope check only when workflow/filter-test files change, with explicit dependencies in that job; do not add Node to ordinary worker checks.
- **GOTCHA:** Do not claim function-level affected analysis. Shared Rust contracts can still select API checks under current dependency policy. Avoid making every worker-only PR install Android tooling.
- **VALIDATE:** Paths matrix: device infra → qualification checks, not API; ordinary Python → worker only; demo → demo build; Postgres → API; qualification Rust source → contracts+worker+API, not web; docs → no app jobs. Include deletions and multiple changed paths.

### Task 10 — Update docs and record gates

- **ACTION:** Add canonical device-qualification runbook and update index/env/development/status/provenance/AGENTS.
- **IMPLEMENT:** Exact install/doctor/run/collect/reset/stop commands; new model key owner; experimental result semantics; provider/model choices and unresolved cloud runtime inputs; raw evidence privacy; limitations; approval packet; record open phase01 browser gate. Document cost formula: active+idle host time × current hourly rate + storage/IP/transfer + observed model charges, with coverage and separate human time. No fabricated numeric quote.
- **MIRROR:** canonical docs and readable comments.
- **IMPORTS:** N/A.
- **GOTCHA:** Current system/source foundation statements must distinguish newly implemented qualification from still-planned phase04 production execution.
- **VALIDATE:** Link/status review after all docs are written; source docs, CLI help and acceptance rules agree.

### Task 11 — Run one offline verification phase

- **ACTION:** Finish authoring, then generate/install declared dependency changes explicitly and execute scoped commands below.
- **IMPLEMENT:** Run contract generation once, scoped checks/builds, offline harness tests and import smoke. Batch corrections; rerun only failed/invalidated checks. Record which checks ran and any blocked device checks.
- **MIRROR:** AGENTS and development.md.
- **IMPORTS:** N/A.
- **GOTCHA:** `just check` and `just build` run unrelated app scopes; do not use them for this phase. Do not claim real-device acceptance from mocked SDK or host tests.
- **VALIDATE:** Commands succeed with no cloud/model access; only expected generated outputs differ; git diff reviewed.

### Task 12 — Execute authorized cloud qualification

- **ACTION:** Once launch packet/spend/access is approved, provision or use the designated host, run the campaign and publish a local qualification record.
- **IMPLEMENT:** Exact approved SKU/region/AMI, `NestedVirtualization=enabled`, encrypted disk, metadata v2, least-privilege identity, no public ADB/emulator ports. Prefer existing SSM access, otherwise restricted operator SSH. Apply bounded host runtime shutdown and explicit operator stop checklist; budgets/alerts alone are not a hard stop. Verify KVM and toolchain before SDK calls. Collect/hash evidence, stop host, confirm retained-disk charges/cleanup and record costs/latencies/mismatches. Retain failures for follow-up.
- **MIRROR:** spec02 acceptance and canonical environment policy.
- **IMPORTS:** Existing AWS CLI/operator access; no new cloud framework dependency.
- **GOTCHA:** No exact AWS region/AMI/model or spending authorization is recorded yet; these are launch-time operator inputs, not blockers to writing the harness. A cloud quote must be current when approved. Model spend requires explicit authorization too.
- **VALIDATE:** Nine expected classifications, interruption/recovery proof, clean reset and artifact transfer hashes; final status is qualified or rejected with evidence. Failed qualification blocks phase04 real execution integration.

## Command interface (implemented; live qualification pending)

All CLI commands below are installed through the existing worker entry point. Root recipes forward arguments, never invent paths, models or budget values.

```sh
# Read-only, no model import or boot
uv run --no-sync --project apps/mobile-worker --frozen mobile-qa-worker device-doctor --profile infra/device-host/profile.toml

# One controlled attempt; request is generated Rust/Pydantic contract JSON
uv run --no-sync --project apps/mobile-worker --frozen mobile-qa-worker device-run --request .private/qualification/request.json

# Campaign uses operator-written nonsecret settings with APK/profile paths and budget/rates
uv run --no-sync --project apps/mobile-worker --frozen mobile-qa-worker device-qualify --config .private/qualification/campaign.toml
```

`just device-smoke request` forwards to device-run; `just device-qualify config` forwards to campaign. `_sdk-run --request ... --result ...` is an internal child interface: do not expose credentials as arguments. Profile contains model ID, time/step/size limits, serial and owned paths; campaign config contains build paths, host billing interval and approved rate/budget metadata. Both are nonsecret. Only actual SDK child launch calls Doppler with `--no-fallback --forward-signals`; offline checks never call Doppler.

## Testing strategy

| Test                     | Input                                                       | Expected                                                  | Edge? |
| ------------------------ | ----------------------------------------------------------- | --------------------------------------------------------- | ----- |
| Strict contract          | version bool/string/2, malformed UUID/hash, extra field     | Reject before touching device                             | Yes   |
| Truth over agent claim   | SDK “success” + created row then missing after ready reopen | failed                                                    | Core  |
| Correct persistence      | Exact unique row survives                                   | passed with full evidence                                 | Core  |
| Creation never happened  | Missing row before reopen                                   | inconclusive                                              | Core  |
| Prerequisite unavailable | Actual503 and unavailable UI                                | blocked; no model call                                    | Core  |
| Ambiguous observation    | Empty/truncated XML, wrong package, missing screenshot      | inconclusive                                              | Yes   |
| Device unavailable       | no KVM, wrong ABI/serial, boot/install timeout              | blocked with setup reason                                 | Yes   |
| Cancellation             | Child ignores TERM or leaves descendant                     | bounded kill, quarantine until recovery, partial evidence | Yes   |
| Concurrent attempt       | Lock already held                                           | reject, do not touch owned or unrelated device            | Yes   |
| Dirty recovery           | Old task/session, stale PID marker                          | explicit reset proof before reuse                         | Yes   |
| Evidence integrity       | symlink escape, partial write, disk full                    | refuse unsafe path, never false finalized success         | Yes   |
| SDK containment          | stdout noise, import dotenv, missing key                    | private logs, clean env/cwd, predictable setup result     | Yes   |
| Usage accounting         | duplicate callback, unknown usage, fallback model           | dedup; explicit coverage; no zero substitution            | Yes   |
| Campaign isolation       | expected verdict present only in oracle                     | not passed to SDK/verifier                                | Core  |
| Existing fake regression | no SDK installed/network blocked                            | existing fixture CLI unchanged                            | Core  |
| CI selection             | added/deleted paths and whole PR list                       | exact affected set                                        | Core  |

No remote-device tests in ordinary pytest. Tests inject command/SDK adapters and temporary filesystem roots. Platform-specific process tests may skip on Windows with a reason; Linux CI must execute them. A dedicated explicit live command owns the device campaign; do not hide it behind pytest collection or default fixtures.

## Validation commands (only after full authoring)

Dependency preparation is explicit; if sdk extra gains the direct callback dependency, update the lock without unrelated upgrades and sync it before checks. Android setup is similarly explicit. Ordinary commands retain `--no-sync` and frozen locks.

```sh
just types
cargo fmt --package mobile-qa-contracts -- --check
cargo clippy --locked --package mobile-qa-contracts --all-targets -- -D warnings
cargo test --locked --package mobile-qa-contracts
cargo build --locked --package mobile-qa-contracts --bin export-contracts
just check-contracts
just check-worker
uv run --no-sync --project apps/mobile-worker --frozen python scripts/sdk_smoke.py
uv run --no-sync --project apps/mobile-worker --frozen python scripts/test_ci_scope.py
bash -n infra/device-host/setup.sh
# Run systemd-analyze verify on the generated unit on its Linux host, before service start.
# Worker wheel build stays private; no PyPI upload.
uv build --project apps/mobile-worker --out-dir .private/builds/worker
# Android fixture only; no test emulator or model call in this build.
cd apps/qa-demo-android
./gradlew --no-daemon :app:assembleGoodDebug :app:assembleBrokenDebug :app:lint
```

Expect strict checks/tests to pass, valid wheel/APKs, and no browser generated diff. The import smoke needs the sdk extra explicitly installed; fake/normal worker tests remain SDK-independent. Run Python fixture_backend syntax/offline HTTP tests in the worker test suite. CI scope test must mirror picomatch semantics, not Python fnmatch. Build dependencies may download during explicit preparation/build; no model/device/cloud workload is allowed in this offline phase.

**Full repository suite:** not required; affected worker/contracts/demo plus changed CI selection checks. CI will still validate broader shared workflow/contract dependencies according to its existing mapping. **Database validation:** not applicable. **Browser validation:** not applicable to this phase; preserve the previous rendered-browser gate.

Manual/live acceptance, after authorization:

- [ ] Exact SDK/toolchain/model/profile/build revisions and hashes recorded.
- [ ] /dev/kvm usable, package manager/screenshot boot readiness confirmed on cloud host.
- [ ] Three good → passed, three seeded defect → failed, three backend outage → blocked; no mismatches hidden.
- [ ] Every passed/failed result has after-create and after-reopen PNG/XML with exact task evidence; blocked has actual prerequisite evidence.
- [ ] Interrupted execution terminates all owned activity; no automatic replay; recovery proves clean state and next good run passes.
- [ ] Required artifact failures cannot produce a green qualification; optional video/usage gaps explicit.
- [ ] Each reset clears prior task/session and external fixture state; failed cleanup quarantines host.
- [ ] Model usage/cost coverage and boot/install/run/reset/transfer durations recorded, including idle costs and partial/unknown values.
- [ ] Evidence copied and checksums verified before resource teardown; instance stopped/terminated as approved.

## Acceptance and completion checklist

- [ ] Tasks 1–11 complete for code readiness; task 12 complete separately for phase qualification. Pending live work is never represented as completed.
- [x] Fake/import commands preserve behavior and their comments remain accurate.
- [x] Rust is the qualification wire source; generated consumers validate boundaries; no duplicated public models.
- [x] SDK source/version integration matches evidence above, no invented methods or default device/model selection.
- [x] Strict lint/types, meaningful tests and scoped builds pass after authoring; no watcher introduced.
- [ ] Real evidence supports each classification, reset and cancellation claim.
- [ ] A developer can implement using this packet and listed mandatory sources; launch-time operator inputs are explicit and not guessed.
- [x] Canonical status/runbook updated, no production/customer capability claimed from demo-only qualification.

## Risks

| Risk                                                | Likelihood       | Impact                   | Mitigation / stop condition                                                                   |
| --------------------------------------------------- | ---------------- | ------------------------ | --------------------------------------------------------------------------------------------- |
| Nested KVM/renderer too slow or unavailable         | Medium           | Blocks real runner       | Hard doctor gate; measure, then change approved host/provider if needed                       |
| Agent fails to create exact task consistently       | Medium           | Feasibility rejected     | Preserve mismatches; improve bounded prompt/profile or reject before product integration      |
| SDK traces/logs claim success without assertion     | High             | False confidence         | Independent verifier and explicit raw trace labeling                                          |
| SDK cancellation leaves processes/device actions    | Medium           | Contamination            | Owned groups+cgroup containment, dirty marker, verified reset, no blind replay                |
| Toolchain image revisions drift                     | Medium           | Irreproducible outcomes  | URL/checksum/revision lock; reject different installed inventory                              |
| Current model selection unavailable/expensive       | Medium           | Blocks/bloats experiment | Explicit runtime profile, approved budget, callback coverage, no automatic provider selection |
| Demo verifier mistaken for generic customer testing | Medium           | Wrong product promise    | Document exact assertion and fixture limits; qualify customer scenarios separately            |
| Shared CI files select too much work                | High today       | Wasted CI                | Narrow infra map with tests; no unrequested whole monorepo redesign                           |
| Cloud budget/credentials absent                     | Known input gate | Live validation pending  | Finish code/offline checks/launch packet first; never mark phase qualified early              |

## Next step

Implement this packet with `prp-implement .claude/PRPs/plans/02-cloud-phone-and-feasibility.plan.md`. Start authoring on a new feature branch from merged main. This plan does not request implementation, commit/push, cloud creation or a live model run by itself.
