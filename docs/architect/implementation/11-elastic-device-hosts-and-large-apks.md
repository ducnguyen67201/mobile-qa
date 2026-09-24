# 11 — Elastic Android hosts and large APKs

Status: local implementation; hosted qualification pending. Local validation and hosted release evidence
are recorded separately in [status](../status.md). This specification does not
establish a measured emulator count or authorize cloud provisioning.

## Purpose and owners

Accept standalone APKs up to **2 GiB inclusive (2,147,483,648 bytes)**, reuse builds
on a qualified Android host, and stop device compute when demand has drained.
Keep Railway API/web, PostgreSQL and private object storage online independently.

`apps/api` owns durable state, authorization, validation and scheduling;
`crates/contracts` owns transport source; `apps/web` owns upload and readiness UX;
`apps/mobile-worker` owns the cache, supervisor and device lifecycle; `infra` owns
images, the EC2 definition and runtime capacity controller. Follow the existing
[contracts](../contracts.md), [Doppler](../environment.md) and
[device qualification](../device-qualification.md) boundaries.

The feature's database tables and runtime access use SeaORM entities, active models,
query builders and schema migrations. Transaction locks remain explicit through
SeaORM so host, claim and multipart fencing retain their ordering guarantees. Legacy
raw-query services outside this feature are a separate repository-wide migration.

## Artifact lifecycle

Object storage holds private immutable APK bytes. PostgreSQL records ownership,
size, full SHA-256, upload state and validation results. Host storage is a bounded
cache; Android installation and customer application data are disposable.

Hosted upload uses 16 MiB parts with at most two concurrent browser transfers.
Multipart uploads are enabled automatically when remote object storage is configured;
there is no separate environment flag. Verify provider signing and CORS before
customer rollout. CORS must permit
`x-amz-content-sha256` and expose `ETag` to the exact application origin.
The API authorizes a specific part and persists its digest and returned receipt.
Resuming verifies the reselected file against every recorded part before sending
additional bytes. Filename and length alone are insufficient. Part hashing uses
bounded native Web Crypto operations rather than loading a complete APK into RAM.

Short-lived signed URLs are explicit capabilities. Browser API mutations still
use the generated SDK, CSRF and Zod boundary. Binary storage requests omit API
credentials and cookies, reject redirects, and accept only approved signing
headers. URLs and tokens must not appear in logs or browser persistence.

Multipart completion queues durable sealing/validation work. The server computes
the whole-file SHA-256 from streamed bytes; a multipart ETag does not establish
that digest. A completed transfer is not yet a validated build. Validation uses
bounded scratch, archive expansion limits and external tool deadlines, and can
recover a leased job after process failure. Incomplete sessions are expired and
aborted through application-owned cleanup because providers differ in lifecycle API
support. Local development retains the bounded streaming API upload path.

Worker build access is authorized against the current lease, including cache hits.
Hosted cache misses use a scoped signed download; local storage uses an authenticated
API stream. Verify byte count and SHA-256 before atomic publication. Namespace by
globally unique app identity and digest, keep active readers pinned, and evict only
complete unpinned entries. Interrupted downloads never become valid cache entries.
Heartbeat and cancellation remain active throughout preparation and cleanup.

Twenty independent cases can reuse one download on one host while installing the
application twenty times. Cache eviction, another host or disk replacement requires
another download. A large APK may require much more guest disk or memory after
installation. Stored profiles keep their qualified size limit; accepting larger
uploads does not retroactively qualify a device for them.

## Host and slot lifecycle

Represent pool policy, physical host/boot, slot and emulator boot as separate
identities. The API owns allowed instance IDs, toolchain digests, qualification
records and app/profile bindings. Host credentials authorize registration and
heartbeats; separate scoped worker grants authorize customer execution.

The supervisor owns Android processes, AVD directories, ports, ADB server contexts
and recovery journals. Executors access a private local device interface. Preserve
the standalone qualification/demo lifecycle and reject unknown processes rather
than attaching to a serial opportunistically. A slot is available only after its
defined reset boundary is proven clean. Failed cleanup quarantines the slot and
retains durable reservations.

Additional slots do not bypass app-wide reservations or exact model qualification.
An app using a shared backend/account remains serialized under existing rules.
Support for a driver/adapter must be explicit; a new supervisor does not make an
unqualified customer-model adapter executable. Existing monthly credit rules remain
authoritative; cloud cold-start and warm idle time are not new customer charges.
The accepted maximum hold includes 30 minutes of preparation and cleanup allowance
per possible attempt. Settlement uses actual server-recorded claimed occupancy,
capped by that hold. Phone preparation/queueing has a 40-minute ceiling; the first
ready event starts the separate 20-minute interactive budget.

The production pool currently admits `android_direct_v1` profiles with generic
execution context. The standalone Minitap demo adapter remains available in its
existing flow; it needs a separate adapter and qualification for supervised slots.

Linux places emulator sockets in a root-owned, nondelegated cgroup. An nftables
output policy denies guest access to metadata, host services, other slots and
private networks, including IPv6. A small fixed-path setuid launcher moves only its
own process into that group, drops privileges permanently, sanitizes the environment
and locks `no_new_privs` before executing the pinned emulator. Public DNS resolvers
are explicit; trusted host processes initiate ADB connections. Root service cleanup
kills remaining emulator children after a supervisor failure. Actual packet tests
on the selected AMI are required before this boundary is considered qualified.

## Runtime power control

OpenTofu provisions one fixed EBS-backed EC2 host, IAM, gateway, Lambda and schedule.
Packer prepares a pinned image. Enabled systemd services register each new boot.
During normal operation, Lambda calls StartInstances and StopInstances; it never
invokes OpenTofu or terminates the instance to reduce idle cost.

An accepted job commits durable demand and a wake hint. Minute reconciliation
repairs lost hints. The controller re-reads API state, checks the exact configured
instance and performs one bounded pass. Lambda concurrency is one, and persisted
operation intents reconcile uncertain AWS responses before another power action.

After the idle grace, the API fences the host as draining. Every execution and phone
claim honors that fence. Active work includes transfer, installation, tests,
evidence, result finalization and cleanup. Stop warm processes, require a fresh
boot-scoped clean receipt, and atomically commit the stop intent after a final
demand check. New demand before commit cancels draining; demand after commit waits
for stop and restart. Missing heartbeats and API outages do not prove a clean drain.

Stopping preserves EBS and its cache, while RAM and running emulators are lost.
Retained storage and the application services continue to incur applicable charges.
Operator quarantine/recovery remains available when automatic cleanup cannot finish;
forced power-off must not manufacture a clean execution receipt.

## Warm policy, resources and qualification

Warm target defaults to zero. A qualified pristine warm slot may be enabled during
explicit timezone-aware windows. Warm policy never retains customer-installed
snapshots across reset boundaries. Draining overrides warm policy.

The initial 2 vCPU/8 GiB one-slot and 4 vCPU/16 GiB two-slot configurations are
benchmark candidates. KVM requires explicitly enabled nested virtualization on a
supported instance and accelerates CPU execution, not graphics or memory capacity.
Heavy applications may require a larger profile and lower concurrency. Record actual
instance, region, AMI, toolchain, profile and workload in qualification evidence.

The pilot provisions 100 GiB encrypted EBS with a bounded cache and free-space
reserve. No inbound ADB/SSH is required. Guest networking must deny access to cloud
metadata, host management endpoints and private infrastructure before customer use;
IMDSv2, security groups and different ADB ports alone do not establish this isolation.
Keep deployment and startup blocked until the actual configuration is qualified.

## Rollout and acceptance

Deploy additive schema first, compatible API second, workers third and browser
last. Keep new host/controller behavior disabled until its acceptance gates pass.
Use the [deployment runbook](../device-host-operations.md) for explicit setup,
recovery and rollback. Do not narrow migrated size columns or delete valid builds
as a rollback shortcut.

- [x] Generated contracts and real route agreement pass locally.
- [ ] Large transfers, validation and cache stay bounded in memory and disk.
- [x] Stale boot, concurrent claim/drain and uncertain AWS outcomes pass local SQL/fake-provider tests.
- [x] Active execution and phone paths participate in the locally tested drain protocol.
- [ ] Guest network and warm reset isolation are qualified on the selected image.
- [ ] Cold start, cached reinstall, drain, stop and restart pass hosted acceptance.
- [ ] Measured capacity, workload latency and total operating cost are recorded.

Kubernetes, ASG/Spot, Azure deployment and a different commercial pricing model
remain separate future decisions. Review the existing plan/report for implementation
details and deviations; this document owns the lasting intended behavior.
