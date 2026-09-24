# PR Review: #17 — Add elastic Android hosts and large APK uploads

**Reviewed:** 2026-09-23

**Author:** ducnguyen67201

**Branch:** `codex/elastic-android-device-farm` → `main`
**Decision:** COMMENT — author self-review; all identified issues fixed for independent review

## Summary

The review covered the full 189-file PR, prioritizing handwritten API, worker, infrastructure, migration, and security-boundary code. The first pass found three unbounded ORM regressions and one Linux-only test failure. All were fixed on the PR branch. No open critical, high, medium, or low findings remain.

The PR intentionally removes raw SQL only from API source files changed by this branch. Existing raw SQL in untouched repository areas remains out of scope for the later repository-wide conversion.

## Findings

### CRITICAL

None.

### HIGH — fixed

- [scheduler.rs](../../../apps/api/src/services/scheduler.rs#L215): execution claims loaded every historical non-canceled run and then queried queued attempts once per run. Claim cost would grow with retained run history. A SeaORM relationship now selects only queued attempts joined to runs for the worker's app, preserves deterministic ordering, and keeps `SKIP LOCKED` fencing for the final claim.

### MEDIUM — fixed

- [credit_billing.rs](../../../apps/api/src/services/credit_billing.rs#L118): credit availability loaded every usage row for a billing period and summed it in Rust. The totals are now computed with typed SeaORM aggregate queries in PostgreSQL.
- [capacity_control.rs](../../../apps/api/src/services/capacity_control.rs#L77) and [device_hosts.rs](../../../apps/api/src/services/device_hosts.rs#L637): each host capacity snapshot scanned queued attempts and reservations across all tenants, and active-work checks loaded the complete reservations table. Queries are now scoped to the host's bound apps, slots, and active owners.
- [test_local_device.py](../../../apps/mobile-worker/tests/test_local_device.py#L245): the macOS renderer test patched the platform after constructing the device. It passed on macOS but deterministically failed on Linux CI because renderer selection occurs during construction. The fixture now establishes the simulated platform first.

### LOW

None.

## Ponytail review

`apps/api/src/services/scheduler.rs:L215-281: shrink: per-run candidate fetch and manual run bookkeeping. One joined SeaORM candidate query grouped by contiguous run, preserving the final lock.`

`apps/api/src/services/credit_billing.rs:L118-143: shrink: duplicated settled/held aggregate pipelines. One typed usage_sum helper.`

`net: -18 lines possible.` Applied during review.

## Validation results

| Check                                                                | Result                                                                |
| -------------------------------------------------------------------- | --------------------------------------------------------------------- |
| Rust format and diff whitespace                                      | Pass                                                                  |
| Rust compile and workspace Clippy with warnings denied               | Pass                                                                  |
| Rust workspace tests on disposable PostgreSQL with configured JDK 17 | Pass: 105/105                                                         |
| Worker Ruff and Pyright                                              | Pass                                                                  |
| Worker tests                                                         | Pass: 243/243                                                         |
| Changed API source raw-SQL audit                                     | Pass: no raw SQL                                                      |
| Web production dependency audit                                      | Pass: no known vulnerabilities                                        |
| Worker and capacity-controller Python dependency audits              | Pass: no known vulnerabilities                                        |
| Existing PR checks before review fixes                               | API, contracts, web, infra, formatting, demo, and scope checks passed |

The retained local test database contains a historical migration name that no longer exists in this checkout, so the first local integration attempt could not migrate it. Validation was rerun successfully against an isolated disposable database, which was removed afterward. A second attempt used an incorrect macOS Java home; the successful run used the installed Homebrew JDK 17 required by APK fixture validation.

## Files in PR

This large PR changes 189 files: 108 source/migration files, 27 test files, 34 infrastructure/configuration files, 13 architecture/plan/report files, and 7 generated contract artifacts. Review coverage was organized as follows:

- API and migrations: multipart lifecycle, artifact validation/delivery, host registration and capacity control, worker leases, task sessions, credit accounting, SeaORM entities, and forward-only schema changes.
- Worker: bounded artifact cache, authenticated delivery, host supervisor, slot IPC, Android lifecycle, isolation checks, and generated transport models.
- Infrastructure: capacity controller, Terraform/Packer/systemd definitions, setuid emulator launcher, nftables policy, deployment validation, and CI scope.
- Web and contracts: resumable multipart transfer, upload recovery UI, generated OpenAPI/Zod/Pydantic outputs, and route/contract tests.
- Documentation: architecture authority, implementation specification, environment/hosting guidance, status, plan, and implementation report.

Generated outputs were reviewed through their owning source definitions and drift/CI checks; handwritten source and trust boundaries received direct full-file review.
