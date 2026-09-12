# Implementation status and evidence

Last reconciled: 2026-09-12. Phase 01 merged baseline: `d137e35` from
[PR #1](https://github.com/ducnguyen67201/mobile-qa/pull/1). Phase 02 local changes are on
`feat/02-cloud-phone-and-feasibility`; see the separate evidence below. This is a
milestone record, not a live CI badge.

## Implemented and observed

- Separate API/web/worker apps; root Cargo workspace and pure contract crate.
- Loco health API, generated browser SDK/types/Zod validation, structured API errors,
  and actual route/OpenAPI agreement coverage.
- SeaORM migration wiring and real local PostgreSQL integration; no product tables yet.
- React navigation and health loading/error/Retry UI; other product routes are placeholders.
- Rust worker JSON Schema, generated Pydantic and deterministic passed/failed/blocked fixtures.
- Minitap 4.0.0 installed as an optional SDK extra; import tested with connections blocked.
- Explicit setup/dev/generation/check/build/smoke commands; no check watchers. CI selects
  affected apps/contracts and cancels superseded PR runs.
- Doppler project `mobile-qa`, scoped locally to `dev`; live CLI process injection verified
  from apps/api. No secret values printed. Vite does not load env files; only the API child
  receives Doppler-fetched values during normal development.

## Validation evidence

The apps/contracts refactor completed 53 unique tests: 21 web, 5 Rust, 26 Python and
1 exporter synchronization test. Strict TypeScript, ESLint, Rust format/Clippy, Ruff,
Pyright, generated drift, builds and runtime smoke passed. Frontend and installed-Python
advisory checks were clean after scoped dependency fixes; the unpublished local worker
is not covered by PyPI advisory lookup.

The affected-CI follow-up verified 23 path cases with picomatch, workflow YAML/command
assertions, API-only integration/build checks and contract-only checks. The Doppler
follow-up passed the 21 affected web tests, typecheck/lint/build, recipe dry run and a
synthetic process isolation/cleanup probe. A subsequent live Doppler metadata assertion
verified the configured project/config without dumping secrets.

[Hosted workflow run for fe3cf30](https://github.com/ducnguyen67201/mobile-qa/actions/runs/34682742656)
completed successfully for scope, API, web, worker and contracts. CodeRabbit status was
also successful when inspected; this is not a claim of a separate human review.

## Open gates and planned work

| Item | Status / next evidence |
|---|---|
| Spec 01 rendered browser/keyboard/Retry/HMR acceptance | Pending: browser tool could not verify admin policy; no bypass attempted |
| Spec 02 real Android/cloud qualification | Local Mac ADB demo verified; full Minitap/cloud qualification pending |
| Spec 03 auth/app creation/APK upload | Planned; current scaffold is unauthenticated |
| Spec 04 worker HTTP leases/runs/evidence reports | Planned; local fake protocol is not a scheduler |
| Spec 05 versioned case/suite/plan editor and approvals | Planned |
| Spec 06 requirements-to-tests generation | Planned |
| Spec 07 regression, retention, production deployment and pilot reliability | Planned |
| Paid pilot/customer validation | No accepted customer app, signed pilot or demonstrated willingness to pay recorded |

Doppler project creation does not provision database/model credentials, deploy a service,
or implement customer-secret resolution. Model/device performance, reset reliability,
HMR timing and production readiness must not be inferred from foundation tests.

The source foundation supports independent device feasibility (02) and app setup (03)
work with agreed ownership. Preserve spec 01's open browser gate during that work.
Historical planning reports remain outside the repository; this portable record replaces
them as the canonical status entry point. Update this file with new evidence rather than
copying old pass counts into every specification.

## Phase 02 implementation and offline evidence

The operator qualification harness, Rust-owned contracts, generated Python models,
controlled good/broken Android demo, pinned tool installer and runbook are implemented
on `feat/02-cloud-phone-and-feasibility`. The qualification protocol is separate from
the fake fixture protocol and the future production worker leases.

Observed locally after complete authoring: 80 worker tests (54 new), 6 contract tests,
10 CI selection cases and the exporter synchronization test passed. Ruff, strict
Pyright, Rust formatting/Clippy, generated drift, worker packaging, network-blocked
Minitap import and both Android APK builds passed. Android lint checks both debug
flavors; expected fixture/version warnings remain. No API/web runtime source changed;
their full suites were not rerun for this phase. Hosted CI has not run on this branch.

At that initial checkpoint, no cloud host, emulator execution, model call or real device
evidence had been produced. The native local-device follow-up below supersedes the local
device status; it does not close Minitap/cloud qualification.
Phase 02 remains unqualified and phase 04 real execution remains gated. Host access or
region/AMI/network, the approved model, current rates, spend and shutdown deadline are
still operator inputs. A launch template is not an approved launch packet. Phase 03
app setup can proceed independently.

See the [implementation report](../../.claude/PRPs/reports/02-cloud-phone-and-feasibility-report.md)
and authoritative [device qualification runbook](device-qualification.md). The PRP plan
remains unarchived until task 12 has real qualification and teardown evidence.

## Hosting direction follow-up

The user selected Railway for the initial application deployment and AWS for a later
migration when needed. [Hosting](hosting.md) records the service layout, portability
constraints and deployment work still to implement. Railway KVM/device execution is
not verified; selecting Railway does not close the phase 02 device-host gate. This
follow-up changes documentation only and has created no provider resources.

## Native local-device follow-up

The runner now has macOS ARM64/Intel and Linux x86_64 host selection, pinned native
Android images, platform acceleration checks and real boot identity for crash recovery.
Explicit device-local setup/build/run commands share the supervisor, assertions and
cleanup with cloud qualification. Default ADB demo navigation uses a real emulator
without model calls; explicit agent mode uses Minitap and Doppler. Reports identify
which navigation path ran. API worker leases and customer APK execution remain planned.

Observed on macOS 15.7 / ARM64: all 100 worker tests, strict Pyright, Ruff, 11 CI scope
cases, both demo APK builds and Android flavor lint passed after the implementation.
Native doctor verified the pinned Android API 35 ARM64 image and Hypervisor.Framework.
Real ADB demo outcomes (reports remain private under `.private/artifacts/local-device`):

| Scenario | Outcome | Reset | Attempt |
|---|---|---|---|
| Good APK | passed | verified_clean | `4b52a6a0-d97a-4b3a-9362-af43bc4c0d67` |
| Broken APK | failed | verified_clean | `06e29435-7652-4a79-bc91-c36ca7c483c7` |
| Backend unavailable | blocked | verified_clean | `76de6d90-8ef9-435f-945c-faad66fc487c` |
| SIGINT during navigation | inconclusive / cancelled | verified_clean | `cb812944-46d7-4497-9ab2-f957a6d41a7e` |

The interruption probe signalled the actual local launcher after navigation started,
waited for its result, and verified the dirty marker was absent and emulator ports
were available. The wrapper replaces itself with the supervisor so it cannot kill
that supervisor while Ctrl-C cleanup is in progress.

The local runs exposed and corrected Mac acceleration-output parsing, timezone/locale
handling, unattended emulator prompts, stale TCP connection checks, fixture-network
startup races and input focus timing. Earlier failed attempts were retained; completed
ADB-only quarantine was recovered explicitly using matching result/profile evidence.
Cold boots in these accepted runs took about 45–46 seconds; full reset took 55–57 seconds.
These timings describe this Mac, not a cloud capacity promise.

No model calls or cloud resources were used. Minitap mode is implemented but its live
11-attempt qualification remains pending, as do HTTP worker leases and dashboard jobs.


## First live Minitap demo

On 2026-09-12, a visible macOS ARM64 demo using Minitap 4.0.0 and OpenAI gpt-4.1
passed the create/save/reopen persistence test and verified a clean device reset.
Credentials were injected from Doppler `mobile-qa / dev_personal`; no key was stored
in the repo. Attempt `ee08c402-43ae-40d3-b462-cac4753356da` recorded 8 model calls,
34,817 input tokens and 941 output tokens, with no unknown usage calls. Private
screenshots, video, SDK traces and the report remain under `.private/artifacts/local-device`.

The preceding attempt `6e3fb69e-e33f-4ebc-91a5-ead5011f7c31` was inconclusive due to
Minitap/LangGraph ToolRuntime incompatibility; its clean reset passed. It made 5 model
calls (22,297 input / 605 output tokens), retained in its SDK child result. Parent
inconclusive reports currently omit usage when the SDK process exits nonzero; that
failure-path usage aggregation remains an accounting limitation.

A typed, version-guarded SDK compatibility subclass now supplies the required config,
tools and execution context without changing installed packages or dependency locks.
Strict Pyright, focused Ruff and all 4 adapter tests passed, including a real offline
Minitap tool execution through LangGraph. See dependencies.md for scope and removal.
This single live demo does not complete the full 11-attempt reliability campaign,
cloud-host qualification, customer APK execution or dashboard job integration.
