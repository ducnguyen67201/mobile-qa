# Implementation Report: Model catalog and qualified capacity

## Summary

The existing immutable registry now has audited definition registration and versioned,
operator-controlled app/profile/purpose assignments. New navigation runs and phone
sessions freeze the assigned model; phone sessions may freeze a distinct structured
authoring model. Protocol 6 advertises a bounded qualified reference set, and both
claim paths skip older jobs that the polling worker cannot execute. Run details show
a computed queue blocker and last compatible heartbeat. The old ADB-only profile
sentinel loads as model-free without modifying private host state.
Queue creation logs the initial reason code, and direct-only runs no longer show an
irrelevant legacy model warning.

## Assessment vs Reality

| Metric        | Predicted | Actual                                             |
| ------------- | --------- | -------------------------------------------------- |
| Complexity    | XL        | XL                                                 |
| Confidence    | 7/10      | Core local path validated; live qualification open |
| Files changed | 25–35     | 44 including generated contracts and docs          |

## Tasks Completed

| #   | Task                                  | Result                                                                                                |
| --- | ------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| 1   | Definition audit and assignment store | Additive migration, immutable revision checks, trusted CLI lifecycle and audit events                 |
| 2   | Freeze new work                       | Navigation and distinct authoring model snapshots; legacy profile reads and direct-only work retained |
| 3   | Qualified worker protocol             | Protocol 6 bounded set with nonsecret host evidence; protocol 5 single reference retained             |
| 4   | Eligible queue selection              | Exact reference/provider predicate before row selection, then existing reservation/fencing            |
| 5   | Queue explanation                     | Read-time reason, wait and compatible heartbeat; run UI and route coverage                            |

## Validation Results

| Level                | Result                                  | Notes                                                                                                                                                                                                |
| -------------------- | --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Generated contracts  | Pass                                    | `just types`, then `just check-contracts` with zero drift                                                                                                                                            |
| Static analysis      | Pass                                    | Rust fmt/Clippy, web typecheck/ESLint, Ruff/Pyright                                                                                                                                                  |
| Rust tests           | Pass                                    | 65 tests on a disposable PostgreSQL database, including new assignment and claim tests                                                                                                               |
| Web tests            | Pass                                    | 153 tests, including the worker protocol queue message                                                                                                                                               |
| Worker tests         | Pass across full pass and focused retry | 176 passed in the consolidated run; six failures were a transient loopback test and fixtures missing the new optional field. The affected 24 tests passed after fixes/retry, covering all 182 cases. |
| Builds               | Pass                                    | Web production bundle and Cargo workspace                                                                                                                                                            |
| Live device/provider | Not run                                 | Explicit operator qualification and recovery gates                                                                                                                                                   |

The retained Compose test database has a migration ledger from another branch.
Validation used a temporary PostgreSQL container, external temporary Loco test config,
and locally generated signed synthetic APKs. It did not reset development data.

## Files Changed

- Rust contracts, migration, registry/queue/session/run services, trusted CLI and route tests.
- Python host profile, worker protocol/admission, authoring model selection and tests.
- Generated browser/worker contracts, run/authoring UI and tests.
- Architecture index, capacity spec, decision record, status and workflow docs.

## Deviations from Plan

- Separate authoring model selection was added during review so the structured purpose
  actually controls phone authoring calls.
- The existing `no-model-adb-demo` host sentinel is accepted only as a model-free
  historical profile to address the worker startup failure without editing private
  operator state.
- Review hardening requires one live phone worker to support both active model purposes
  and records execution protocol heartbeats separately from phone polls.
- `max_calls` is versioned assignment metadata but is not yet a runtime stop
  condition. Existing workflow budgets remain in effect; this limit must be enforced
  before presenting it as a hard spend cap.

## Remaining Gates

- An operator must verify the qualification evidence against the pinned SDK/runtime
  and physical device, then perform a real device/provider campaign before rollout.
- The existing recovery-required reservation must be resolved through verified
  operator recovery; this implementation does not release it automatically.
- The source change has not been deployed or validated against a live device/provider.

## Tests Written

- Real PostgreSQL tests for assignment revisions, frozen snapshots, oldest-compatible
  claim selection and queue reasons.
- Worker tests for qualified host set parsing and the historical ADB sentinel.
- Browser tests for recovery queue copy and distinct authoring model readiness.

## Next Steps

- Run the explicit operator qualification campaign before rollout.
- Enforce the recorded model-call ceiling before using it as a commercial or safety limit.
