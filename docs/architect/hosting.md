# Hosting: Railway first, AWS later

Status: Railway application hosting was selected on 2026-09-12. Local deployment
definitions now include the application image and a separate fixed EC2 device pool.
Live deployment and device qualification remain release gates. Moving the application
services to AWS remains a later decision based on measured cost and operations.

## Initial deployment shape

| Component                             | Initial placement                                               | Reason / boundary                                                                                       |
| ------------------------------------- | --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Rust API and compiled React dashboard | One Railway application service                                 | Loco already has same-origin static SPA configuration; keep separate source apps but one deployed image |
| PostgreSQL                            | Railway database service                                        | API connects privately; Python never receives database credentials                                      |
| APKs and report artifacts             | Railway private Buckets for hosted APKs; reports follow spec 04 | S3-compatible adapter exists; hosted round-trip and provisioning remain open                            |
| Python worker and Android emulator    | One separately qualified Linux x86_64 KVM host                  | Current harness owns the emulator locally; Railway device execution is not qualified                    |

Railway supports container services and monorepos. Its documentation describes
non-privileged containers, and a Railway employee's nested-virtualization support
answer says it was unsupported at the time of that answer. That support answer is
older; it does not prove what a particular future or private runtime permits. We have
not verified usable `/dev/kvm` or the required lifecycle controls on Railway. Do not
assume packaging the emulator in Docker supplies those host capabilities.
[Services](https://docs.railway.com/services),
[runner limitations](https://docs.railway.com/guides/github-actions-runners),
[nested virtualization support answer](https://station.railway.com/questions/enable-nested-virtualization-support-in-de9f4c0f).

The phase 02 doctor requires usable KVM on Linux; native Mac development uses Apple's
Hypervisor.Framework with a matching image. Railway can become a device-host candidate if it confirms support
for the selected runtime and the full boot/reset/cancellation campaign passes there.
Otherwise use an existing suitable host or select a separate device provider; this
hosting preference does not select or authorize an AWS device instance. A managed
remote phone would require a separate adapter assessment because today's harness owns
a local emulator and owned process groups (with systemd containment on cloud Linux).

## Application deployment boundaries

1. Build one portable application image: compile the web assets and Rust binary with
   pinned tools, preserve the expected static-asset paths, and run only the application.
   Keep repository-root build context because the API depends on `crates/contracts`.
   Source ownership remains `apps/api` and `apps/web`; no folder move is needed.
2. Production configuration binds `0.0.0.0` and reads platform `PORT`; the image defaults
   to port 5150. Railway health checking uses `/api/health`. Verify SPA/API same-origin
   behavior after deployment.
3. Supply a config-scoped Doppler service credential through the platform secret store.
   Fetch application secrets only at runtime through Doppler; no env files or secrets
   in image builds. Configure the Railway-private database connection in the appropriate
   Doppler config through an authorized secret operation.
4. Configure Railway deployment watch paths for application inputs: both source apps,
   shared Rust dependencies, workspace/toolchain locks and the deployment image/config.
   Worker/device/docs-only edits should not rebuild the application image. Database
   migrations stay in the API deployment scope. Railway watch paths select deployments;
   GitHub path filters continue to select tests independently.
5. Complete the coherent deployment implementation before running scoped checks/builds.
   Deploy and verify only the capabilities that exist; publishing a scaffold does not
   implement authentication, uploads or the production worker protocol.

Railway documents repository-root builds for shared monorepos and deployment watch
paths in its [monorepo guide](https://docs.railway.com/deployments/monorepo).

Phase 02 remains an operator campaign without an API connection. The implemented
phase 04 worker initiates authenticated HTTPS claim/heartbeat/result calls to the API.
No public ADB port or remote database access is needed. The new supervisor restricts
pool admission to qualified generic direct execution; the existing standalone Minitap
demo needs a separate qualified adapter before it can use supervised slots.

## On-demand device infrastructure

[Spec 11](implementation/11-elastic-device-hosts-and-large-apks.md) defines the fixed
EC2 device pool, large build delivery and runtime start/drain/stop controller.
`Dockerfile.api` packages the same-origin application; `railway.toml` selects its
build and health check. OpenTofu under `infra/aws/device-pool` defines device
resources independently of Railway. The controller defaults to disabled. Use the
[device operations runbook](device-host-operations.md) for rollout and required
qualification; checked-in configuration does not establish a live deployment.

API production binding accepts the platform PORT on all interfaces. Runtime secrets
still come from Doppler. The device image has its own SDK and qualification gates,
and the host cache does not change private object storage as the source of APK bytes.

## Later application migration to AWS

Keep portable container images, ordinary PostgreSQL/SeaORM migrations, generated HTTP
contracts, S3-compatible artifact access and Doppler configuration. Avoid provider
identifiers in domain records. Move the application image first or move device capacity
independently; the worker and app do not need to migrate together.

A database move still needs a tested backup/restore or replication procedure, write
cutover and rollback plan. Artifact access, DNS, secrets and worker API endpoints also
need deliberate cutover. Portability reduces application rewrites; it does not make
migration a one-click operation. Do not add Kubernetes, multi-cloud abstractions or
another queue solely to prepare for a possible move.

See [device qualification](device-qualification.md) for the separate hardware gate and
[current status](status.md) for observed implementation evidence.
