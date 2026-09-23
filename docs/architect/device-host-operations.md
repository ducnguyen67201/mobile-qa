# Device host deployment and operations

Status: local implementation; deployment qualification pending. This runbook is not hosted
acceptance evidence. [Spec 11](implementation/11-elastic-device-hosts-and-large-apks.md)
owns the architecture; [status](status.md) records verification.

## Release inputs

Prepare an approved account/region, explicit supported nested-virtualization SKU,
immutable application/controller base-image digests, approved AMI, API origin and
operator-owned pool/host/profile identifiers. Price the whole topology: running
compute, retained EBS, public IPv4, logs, application/database hosting, artifacts,
network and model usage. Account budgets and alerts complement safe drain; a stopped
host still retains billable disk.

Use a reviewed release archive containing committed source and lockfiles. Build
artifacts must exclude private state, credentials, customer APKs and local caches.
Android SDK license acceptance is an explicit image-build input. Do not run an AMI
build or apply infrastructure until the user authorizes the priced resource plan.

Doppler remains runtime secret authority. Supply narrowly scoped API, host and
controller configurations independently. Platform stores hold bootstrap tokens;
OpenTofu receives secret references, never secret values. Do not create env files,
fetch credentials into files, or pass them as shell arguments. Ordinary checks do
not need Doppler or AWS credentials.

## Deployment sequence

1. Review and apply additive database migrations through the normal API release.
   Back up first. Preserve old worker and upload compatibility during rollout.
2. Deploy the same-origin API/web image with platform PORT, health checking and
   sufficient validation scratch. Confirm the private bucket's actual multipart,
   signed URL and CORS support. Enable only exact browser origins and expose part
   receipts. Verify server-computed hashes and cleanup of abandoned transfers.
   Multipart uploads are enabled automatically for remote storage. Complete these
   checks before opening the deployment to customer uploads.
3. Register qualified device profiles and prepare pool/host/slot UUIDs and app
   bindings. Keep the pool registration document pending until the actual EC2
   instance ID exists; do not invent an instance ID.
4. Build and review the host AMI and controller image. In a separately authorized
   staging campaign, prove guest denial of metadata, host and private infrastructure
   access for the exact image. The fixed pool's apply gate requires that prior
   evidence. Configuration flags or placeholder evidence strings are not qualification.
5. Review an OpenTofu plan using a protected encrypted remote state backend. Inspect
   IAM scope, nested virtualization, encrypted disk, replacement protection, image
   digest, no inbound device ports and all retained-resource charges. Keep controller
   disabled during registration and qualification. Initial instance creation itself
   starts paid compute; plan its operator stop even before controller enablement.
6. Register the pool through the trusted API operator task using the instance ID
   returned by OpenTofu. Install the approved root-owned network qualification
   manifest at `/etc/mobile-qa/network-isolation.json`, matching the exact toolchain
   digest and recorded controls, then start/restart the host service. Startup remains
   blocked until both registration and qualification evidence are available. Verify
   new-boot registration, device doctor and cold ownership/cleanup. Keep warm target
   zero and one slot until their independent qualification passes.
7. Enable the capacity controller, verify authenticated immediate hints and scheduled
   reconciliation, and exercise a complete upload/run/report/drain/stop/restart cycle.
   Test demand arriving during stopping and AWS responses whose outcome is uncertain.
8. Record measured resource utilization, timings, cache reuse and actual charges.
   Raise slot count only for passing host/profile/workload combinations.

## Failure handling

| Symptom                         | Required response                                                                            |
| ------------------------------- | -------------------------------------------------------------------------------------------- |
| API unavailable                 | Keep host fenced according to last durable state; controller must not infer clean drain      |
| Host heartbeat absent           | Investigate or quarantine; retain reservations until verified recovery                       |
| Startup exceeds budget          | Quarantine and alert; avoid unbounded boot/restart loops                                     |
| Download interrupted            | Renew authorized capability and restart/resume bounded transfer; never publish partial bytes |
| Cache full                      | Release expired partials/evict unpinned entries; do not delete active files or journals      |
| Multipart abandoned             | API sweeper aborts provider upload and records outcome; retry failures                       |
| Evidence/finalization pending   | Keep host active until durable completion and cleanup acknowledgment                         |
| Unknown Start/Stop response     | Reconcile observed power and operation journal before another power transition               |
| Cleanup cannot prove stop/reset | Quarantine, preserve evidence/reservations and require operator recovery                     |

An incident may require an explicitly authorized power-stop to contain spend. Fence
new claims first, retain affected attempt state and recovery evidence, and inspect
actual AWS state. A forced stop is not a passing cleanup receipt. Do not delete dirty
journals or reservations to regain availability. Before restarting, ensure no older
controller operation can still act on the previous generation.

### Resolving an uncertain power operation

Use the API's trusted `execution` task with `action:show-pool` to inspect the durable
snapshot. Pool maintenance actions also require the trusted operator's `actor` UUID.
Do not pass any token on the command line; `register-pool` and credential rotation
read separately injected host/control tokens.

1. Pause admission with `action:pause-pool` and the pool UUID.
2. Disable both the EventBridge schedule and Lambda reserved concurrency, then
   wait for the last invocation to finish. Inspect actual EC2 state independently.
3. If a pending operation cannot be reconciled, quarantine the host and use
   `action:resolve-power-operation` with `actor`, `pool`, exact `operation` UUID,
   current `control-version`, independently observed `power:stopped` or `power:running`,
   `controller-disabled:true`, and an incident `evidence` reference. The command
   uses a version comparison and preserves quarantine and reservations.
4. Resolve affected attempts through the existing recovery procedure. Only use
   `action:recover-host` when power is verified stopped, no operation remains,
   and all reservations have been resolved. Inspect `show-pool` before resuming.

API recovery does not erase local ownership evidence. Keep the host service disabled
while investigating a prior `owner.json`, `worker.json`, device `dirty.json` or socket.
Preserve those files with the incident evidence, verify that owned process groups,
the emulator cgroup and slot ports are empty, and resolve the corresponding API
attempts first. Then explicitly retire the affected slot state directory before a
new service boot. The existing standalone `device-recover` command does not resolve
new supervisor worker journals. Automatic restart intentionally refuses ambiguous
ownership; deleting journals to suppress a recovery error is not a safe recovery.

`register-pool` reads a bounded JSON `file` matching generated `PoolRegistration`.
`rotate-host-token` and `rotate-control-token` have separate token scopes. All
actions run through the API operator CLI (`cargo loco task execution action:<action>`) in its explicitly selected environment;
they do not call AWS power APIs themselves.

### Supervisor and capacity reports

The systemd unit starts `mobile-qa-worker host-supervisor --config
/etc/mobile-qa/host.toml`. The file references the qualified device profile,
root-owned network evidence and server-approved slot resources. It contains no
credentials. Slot grants arrive through the authenticated host protocol.

After an authorized measurement campaign, run `mobile-qa-worker
host-capacity-report --campaign /absolute/recorded-campaign.json --output
/absolute/new-report.json`. This evaluates supplied measurements and leaves
`approved_slots=0` for operator review; it does not run a device or automatically
approve capacity. Include cold/cache/warm/reinstall/heavy-app/failure/drain samples,
latencies, memory/CPU/disk measurements and actual full-topology charges.

## Rollback and rotation

Pause new admission and drain existing work before disabling the controller. Preserve
metadata and bytes for already accepted large builds; do not roll BIGINT back to
INTEGER or reintroduce a validator that cannot read accepted records. A rollback
must preserve resume, retrieval and validation for existing multipart uploads.

Rotate host/control credentials in their separate scopes. Reject revoked identities
for new claims while reconciling already accepted results through durable receipts.
Changing an AMI or user-data may replace an instance: review the plan and retained
volume handling explicitly. Never remove destruction protection as an idle-cost
mechanism. A future ASG has different cache/termination semantics and needs its own
lifecycle design.

## Evidence required before a production claim

Local tests use fake device/cloud providers and do not prove KVM, graphics, APK
installation, guest network isolation, warm reset or real-world density. Record
these as separate operator campaigns, including failures and the tested limits.
Document all cloud/device/model runs and their authorization; retain no credentials
or customer binaries in the repository.
