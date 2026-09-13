# Plan: Visual task sessions

## Summary

Implement phase 06's primary task session: open an uploaded app, show its real screen,
submit a plain-language goal or selected control, and let Minitap act. Keep the
existing saved-test workflow available as advanced functionality. Scope this first
vertical slice to task sessions; batch generation and broader discovery remain planned.

## User story and UX

As an app owner, I want to describe what to try and watch the phone, without first
writing cases, suites or release plans. Before: many manual fields. After: Open app,
a phone preview, one task input, Run task, Stop, and optional Save as test.
Auto-select the latest validated build and the sole compatible qualified worker
profile. Ambiguous configuration is shown explicitly. Never show a simulated screen
as a real device. Capture click context without silently sending a tap.

## Metadata

Complexity: XL. Source: docs/architect/implementation/06-test-generation.md.
Baseline: phase 05 currently exists as local uncommitted work; preserve it on
codex/06-task-sessions. GitHub PR #5 is phase 04, not evidence of a phase 05 merge.

## Mandatory reading / patterns to mirror

- crates/contracts/src/execution.rs, execution_api.rs: Rust serde/Schemars/Utoipa DTOs.
- apps/api/src/services/execution_store.rs: parameterized SeaORM SQL, field/decode/json.
- apps/api/src/services/scheduler.rs: app lock, device advisory lock, durable resource
  reservation, fenced worker lease. Expiry quarantines rather than releasing a phone.
- apps/api/src/controllers/runs.rs and worker.rs: browser Session vs bearer Worker.
- apps/api/tests/test_library.rs and tests/support: actual authenticated route tests.
- apps/mobile-worker/src/mobile_qa_worker/execution/client.py: bounded same-origin HTTP.
- qualification/device.py, process.py, sdk_adapter.py: owned disposable AVD, host lock,
  process-group cleanup, Doppler-isolated Minitap TaskRequest goal seam.
- apps/web/src/api/test-library.ts and pages/TestLibrary.test.tsx: generated SDK/Zod,
  React Query, synthetic HTTP DOM interaction tests.
  No new external library API required; reuse the pinned internal Minitap seam.

## Files to change and tasks

1. Contracts: add task_sessions.rs/task_sessions_api.rs; register browser/worker exports.
   Include options, open/get/stop, goal submission, frames/controls, history, worker
   claim/update, and optional save draft. MIRROR execution contracts. Validate budgets
   and selection identity on the server; never trust generated static types alone.
2. Storage: migration 000006 plus registry; sessions/tasks/frame evidence with tenant
   scope, immutable task goals and idempotency IDs. Extend existing reservations to
   permit exactly one attempt or session owner. MIRROR migration 000004.
3. API: task_sessions service/controller; qualified app-scoped worker selection;
   exclusive claims, fenced heartbeat, canceled/expired sessions, private screen,
   no blind replay. MIRROR scheduler and browser controllers. Add route registration.
4. Worker: explicit task-worker command, existing client and pinned SDK seam. Own the
   AVD for a bounded session; keep captures and heartbeats flowing during SDK execution;
   stop SDK before confirming cleanup. Persist dirty markers across crashes. Do not
   send API credentials to the model process. MIRROR execution runner/process tools.
5. Browser: TaskSession page and generated transport; direct Open app action from app
   and Tests; phone/control selection, goal, progress, stop and optional save. Render
   all missing-device/setup/error states honestly. Preserve existing drafts and URLs.
6. Tests/docs: contract/route/worker/DOM boundary and lifecycle coverage; update owning
   phase spec/status and report with actual evidence. No ordinary device/model calls.

## Testing strategy and acceptance

- A task can run without a case, approval, suite or plan.
- Defaults require no extra choice when build/profile are unambiguous.
- Cross-tenant access, stale selection, repeated submission, unavailable device,
  duplicate claims, concurrent reservations and expired authority are rejected safely.
- Cancel stops device/model work before resource release; uncertain cleanup quarantines.
- Screen/goal context reaches Minitap; completion is not labeled a verified test pass.
- Save produces an incomplete editable draft without automatic approval.
- Real acceptance: offline sample APK; goal saves text and observes it after restart;
  a different goal proves the input is honored. Browser admin restriction remains:
  no alternate access to bypass it; disclose rendered acceptance if unavailable.

## Validation commands

Finish all code/test/config before generation/checks. Then just types; just format;
just check-contracts; just check-web; just check-api; just check-worker; just build.
Use one consolidated check window and rerun only failed or invalidated checks.
Actual route tests use disposable PostgreSQL. Real device/model execution is explicit,
Doppler-injected and separate from ordinary checks. Do not restart the user's existing
servers until the new implementation is complete and verified.

## Completion

Write .claude/PRPs/reports/06-task-sessions-report.md. Archive this plan only when
all required work and acceptance are complete. Record unsupported/gated behavior
explicitly rather than marking phase 06 complete from fake evidence.
