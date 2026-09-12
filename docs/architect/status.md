# Implementation status and evidence

Last reconciled: 2026-09-12. Runtime source baseline: `fe3cf30` on
[PR #1](https://github.com/ducnguyen67201/mobile-qa/pull/1). This is a milestone record, not a live CI badge. Spec 03 is implemented locally on
`codex/03-app-setup`; hosted/rendered acceptance is still open.

## Foundation evidence before spec 03

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

## Historical foundation validation

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
| Spec 02 real Android/cloud qualification | Planned; no cloud host, emulator execution, model calls or device evidence yet |
| Spec 03 auth/app creation/APK upload | Local implementation verified; hosted Railway round-trip and allowed rendered acceptance remain open |
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
Current implementation reports are linked below; this remains the canonical status entry point.

## Spec 03 implementation evidence (2026-09-12)

Implemented: Google-only sign-in and app cookie sessions with revocation/origin/CSRF controls;
tenant-scoped apps/environments; private local storage and a hosted S3-compatible
adapter; real bounded Android metadata/signature validation; immutable persisted build
history; Mantine dashboard; explicit reference/observation/cleanup tasks.

Observed on the working branch: 56 web tests, 15 Rust tests and the exporter helper
regression pass; TypeScript, ESLint, Rust format/Clippy, generated drift and both builds
pass. The signed synthetic APK HTTP smoke validates bytes/hash/metadata, rejects a
foreign organization and retrieves the same build after API restart (0.145s completion
for this tiny fixture). Foundation smoke passes with a 0.621s warm built API startup.
These timings are local synthetic measurements, not customer APK or hosted SLAs.
Worker sources/contracts are unchanged; the existing fake and import-only SDK smoke
remain the only worker evidence. No device execution occurred.

The original shadcn GAN source review improved 7.27 → 7.87. After the user's
maintenance feedback, Mantine 9.6.1 replaced all copied UI primitives, `cn`, the
mobile hook and Tailwind configuration. A fresh source-only review passed after
fixing drawer close labels, resize/scroll-lock behavior, navbar scrolling and long
text wrapping. The earlier numeric scores do not evaluate this new implementation.
The post-refactor frontend validation passes: 56 tests across 5 files, TypeScript,
ESLint, production build, clean pnpm audit and diff whitespace checks. New DOM
coverage checks drawer Escape/focus return, account-menu logout, form submission
payloads and reopening the latest environment revision. State now uses Mantine
useDisclosure/useForm with a dedicated useApkUpload workflow hook; generated transport
and server reconciliation remain authoritative. No browser was
opened. Vite retains a nonblocking chunk warning (749.13 kB / 227.28 kB gzip main JS;
235.58 kB / 34.68 kB gzip CSS). RustSec flags unpatched transitive `rsa` advisory RUSTSEC-2023-0071;
App sessions use HMAC JWTs; Google identities use RSA public-key verification. See [dependencies](dependencies.md).

Hosted acceptance still requires authorized Railway bucket/streaming round-trip,
private access policy, abandoned multipart lifecycle and parser resource isolation
checks. Browser/keyboard acceptance remains blocked by the existing admin-policy
restriction; no alternate access was attempted. Device readiness remains `not_checked`
and overall execution readiness remains false.

Implementation details, deviations and final checks:
[Spec 03 report](../../.claude/PRPs/reports/03-app-setup-report.md).

### Google-only follow-up (2026-09-12)

Google Identity Services replaces password authentication. A second migration removes
password hashes and revokes earlier sessions while retaining users and product data.
Access remains invitation-only. The API verifies the Google signature, issuer,
audience, expiry, verified email and a one-use browser-bound nonce; linked identities
use the immutable Google subject. The web uses the official Google button and a
separate sign-in hook; no password form or reset-password task remains.

Current checks: 57 web tests pass; TypeScript, ESLint, Vite build, Rust format/Clippy
and generated drift pass. Google route tests cover rejected claims/signatures,
missing browser binding, replay, expired challenges and unauthorized identities.
All 16 Rust tests pass. The HTTP smoke completes synthetic Google sign-in, APK
validation and persisted retrieval after API restart (0.147s fixture finalization);
foundation smoke passes (0.211s warm API startup). These are local synthetic results. Frontend audit is clean; the existing
unpatched RSA advisory remains. Main JS is 752.46 kB / 228.30 kB gzip with the existing
nonblocking size warning. Prior baseline counts above are historical.

Real Google account consent is unverified: configure a Google web client and its
origins, and inject GOOGLE_CLIENT_ID through Doppler. No Google/cloud configuration
or browser access was changed during this implementation.

### Workspace onboarding follow-up

Source now supports Google registration without invitation, approval-gated workspace
creation, multiple owned workspaces and URL-based selection. Migration 000003 adds
account approval and preserves existing access. Scoped listing, invalid workspace
links, app/workspace mismatches and ownership checks are covered by new route/DOM
cases. Validation passes: 65 web tests, 17 Rust tests, TypeScript, ESLint,
Rust format/Clippy, both builds and generated-contract drift. The synthetic APK HTTP
smoke verifies tenant denial and persistence after API restart (0.251s finalization).
Frontend dependency audit is clean; the existing RustSec RSA advisory and Vite bundle
size warning remain. The earlier invitation-only descriptions above are historical.

A local Google OAuth web client has now been configured through the user-authorized
browser flow, with GOOGLE_CLIENT_ID injected through Doppler. Real Google sign-in
was verified against the local app. This supersedes the earlier unverified Google
setup and browser-access notes; hosted storage and device acceptance remain open.

The restarted local app was verified through real Google sign-in: the existing
account lands on its workspace URL, the chooser shows its owned workspace, and
the approved Create Workspace form renders. Creation/switching across two workspaces
and pending-account gating are covered by automated route and DOM tests.
