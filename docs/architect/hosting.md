# Hosting: Railway first, AWS later

Status: application hosting direction selected by the user on 2026-09-12; deployment
configuration and live deployment are not implemented. AWS is the later migration
target when measured capacity, cost or operational needs justify it. Device hosting
remains a separate qualification decision.

## Initial deployment shape

| Component                                  | Initial placement                                               | Reason / boundary                                                                                       |
| ------------------------------------------ | --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Rust API and compiled React dashboard      | One Railway application service                                 | Loco already has same-origin static SPA configuration; keep separate source apps but one deployed image |
| PostgreSQL                                 | Railway database service                                        | API connects privately; Python never receives database credentials                                      |
| APKs and report artifacts                  | Railway private Buckets for hosted APKs; reports follow spec 04 | S3-compatible adapter exists; hosted round-trip and provisioning remain open                            |
| Python Minitap worker and Android emulator | One separately qualified Linux x86_64 KVM host                  | Current harness owns the emulator locally; Railway device execution is not qualified                    |

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

## Deployment work to implement

1. Build one portable application image: compile the web assets and Rust binary with
   pinned tools, preserve the expected static-asset paths, and run only the application.
   Keep repository-root build context because the API depends on `crates/contracts`.
   Source ownership remains `apps/api` and `apps/web`; no folder move is needed.
2. Adapt production binding and port to Railway. Current production config binds
   `127.0.0.1:5150`, so the scaffold is not directly ready for public container routing.
   Configure a health check and verify SPA/API same-origin behavior after deployment.
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

Phase 02 remains an operator campaign without an API connection. In phase 04 the
separate worker will initiate authenticated HTTPS claim/heartbeat/result calls to the
Railway API. No public ADB port or remote database access is needed for that design;
the production protocol itself remains planned.

## Later AWS migration

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
