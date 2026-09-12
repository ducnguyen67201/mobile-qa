# 02 — Cloud phone and execution feasibility

Status: harness implemented; offline checks passed; live qualification pending. No paid infrastructure provisioned. Depends on: minimal source setup. Blocks: real execution in spec 04. Owns: `apps/mobile-worker/`, device-host configuration under `infra/`, qualification records.

Detailed [phase 02 implementation plan](../../../.claude/PRPs/plans/02-cloud-phone-and-feasibility.plan.md) prepared against merged scaffold `d137e35` on 2026-09-12. Implementation and offline validation are recorded in the [report](../../../.claude/PRPs/reports/02-cloud-phone-and-feasibility-report.md); cloud qualification remains pending. The packet specifies a controlled demo APK, an independent persistence verifier, strict qualification contracts, process cleanup, affected CI and an authorization gate before paid execution. This document remains the architecture authority.

## Decision

Start with **one Android emulator on one Linux cloud host**, with the Python worker on that same host. The customer uploads an APK; they do not install a local testing framework. This is a virtual phone, and reports must identify it as an emulator.

The application will start on Railway, with AWS as a later migration target; see
[hosting](../hosting.md). This does not resolve the emulator host: Railway KVM support
is unverified, and the current doctor requires it.

Optional separate device-host candidate: an AWS EC2 M8i instance with nested virtualization explicitly enabled. Start sizing evaluation at 4 vCPU/16 GiB for one emulator plus worker; this is a resource hypothesis, not a throughput guarantee. Before creation, confirm the exact instance SKU, region capacity, virtualization support, current price and authorized spend. AWS documents nested virtualization on selected C8i/M8i/R8i families; do not assume every EC2 instance supports it. [AWS launch note](https://aws.amazon.com/about-aws/whats-new/2026/02/amazon-ec2-nested-virtualization-on-virtual/), [configuration instructions](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/amazon-ec2-nested-virtualization.html)

An existing qualified KVM-capable host is equally suitable. A managed device provider is a fallback to evaluate if self-hosted qualification is too costly; it must expose the control, reset, evidence and isolation capabilities our adapter needs. Do not assume all Appium/device-farm sessions support this SDK's device interface.

## First test, before generation

Use a controlled demo app with an x86_64-compatible APK and a known requirement: “Create a task, close/reopen the app, and verify the task persists.” Supply a resettable account if it uses a backend. The assertion checks the exact unique task after reopening; a success toast alone is insufficient.

Prepare three scenarios: a known-good build, a seeded persistence defect, and an unavailable login/backend prerequisite. The expected classifications are pass, fail and blocked. Run each three times from reset state as an early engineering check. Preserve every result; this is not the larger pilot reliability evaluation.

## Cloud boot requirements

1. **Provision host:** Linux x86_64, encrypted disk, restricted operator access, instance identity with narrow artifact permissions, and explicit nested virtualization. Keep ADB/emulator control ports off the public network.
2. **Verify acceleration:** check usable `/dev/kvm`, user permissions and the pinned emulator's acceleration diagnostic. Fail preflight if acceleration is unavailable. Linux emulator acceleration uses KVM. [Android acceleration documentation](https://developer.android.com/studio/run/emulator-acceleration)
3. **Install pinned tooling:** Android command-line tools, platform tools, emulator and one system image; Python/uv and the locked worker. Record package versions, image revision and SDK license acceptance. Run the emulator directly under a host service initially; no Kubernetes or nested Docker setup is necessary.
4. **Create the device:** candidate Android 15/API 35 Google APIs x86_64 image, 1080×1920, 420 dpi, portrait, en-US, UTC. Qualify these settings rather than relying on defaults. An ARM-only native APK needs a compatible rebuild or a different qualified device; do not promise transparent translation.
5. **Boot headlessly:** use the launch interface supported by the pinned tools, disable restoration of contaminated user snapshots, and select a verified headless graphics mode. Wait for ADB, Android boot completion, package-manager readiness and a successful screenshot; use a hard timeout.
6. **Install and preflight:** verify APK checksum, package, minimum SDK and ABI; install, launch, and check access to the approved test backend. Installation/readiness errors are infrastructure or prerequisite results, not failed business assertions.
7. **Execute:** run the Python adapter against the explicit local device serial. Configure the model once without interactive prompts. Enforce a wall-time budget and capture trace events, screenshots, logs and available video. Save missing-evidence reasons.
8. **Reset and verify:** clear device/app state using a qualified clean-image process, reset backend fixtures separately, and prove no prior test data/session remains. A fresh emulator alone does not reset remote customer data.
9. **Recover and stop:** kill an interrupted attempt's process tree, quarantine uncertain resources, reconcile cleanup, and return only verified-clean capacity. For the pilot, manually start/stop the host with an operator checklist; automate pool scaling later.

Android's current command-line page documents a transition toward `android emulator`. Pin the toolchain and validate the exact invocation during implementation rather than treating an old command snippet as an evergreen provisioning script. [Android command-line documentation](https://developer.android.com/studio/run/emulator-commandline)

## Adapter experiment

Use the installed SDK's actual local-device interface. Inspect upstream code/documentation for event delivery, cancellation and artifact access before defining wrapper calls; names in our specs are product contracts, not claimed mobile-use APIs. Current installation docs specify Python 3.12+ and local Android access through ADB. [Minitap installation](https://www.minitap.ai/docs/mobile-use-sdk/installation)

Keep mobile-use responsible for navigation. Add our verifier and evidence normalization around it; do not put another agent in charge of every tap. Inspect licensing/dependency notices at the exact pinned revision and preserve required notices.

## Acceptance / stop conditions

- Every scenario has build checksum, device/image versions, case ID, attempt ID, expected versus observed result and usable evidence.
- Seeded failure never becomes a pass merely because the agent reports completion. Missing prerequisites remain blocked; ambiguous evidence is inconclusive.
- Repeated reset works; interrupted execution stops before device/account reuse.
- Record cold boot, install, run, reset and artifact-upload duration; model usage and estimated host cost per run, including idle time.
- If the APK cannot install, reset cannot be verified, or evidence cannot support the assertion, stop UI expansion around real runs and resolve or change the device/runner approach.

iOS and physical-device support remain outside this first qualification. Public Minitap pages differ on physical-iOS support; validate the pinned SDK and provider separately before expanding the product promise.
