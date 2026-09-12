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
| Spec 02 real Android/cloud qualification | Harness implemented and offline checks passed; live host/device/model qualification pending |
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

No cloud host, emulator execution, model call or real device evidence has been produced.
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
