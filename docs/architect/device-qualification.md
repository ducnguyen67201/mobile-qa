# Android runner qualification

Phase 02 is an operator experiment against our controlled demo APK. It is not the
phase 04 HTTP worker lease protocol, and passing this experiment does not establish
customer-app support or production reliability. The existing fake/import commands
remain separate. Implementation/validation status is recorded in [status](status.md).

## What the harness does

`device-doctor` reads a profile and checks the native host/image architecture,
usable KVM on Linux or Hypervisor.Framework on macOS, pinned package
revisions and disk. `device-run` owns one emulator-5554 attempt; `device-qualify` runs
three good/broken/backend503 cycles, an interruption probe after navigation starts,
and one recovery run. The campaign oracle never enters the worker request or prompt.

Minitap enters and saves a unique task. Our code observes its exact package-scoped
row, force-stops/reopens the app and checks persistence. Missing evidence or failed
navigation is inconclusive. Backend unavailability is blocked. A verified saved task
that disappears is failed. SDK completion and trace `_PASS` suffixes are diagnostic,
not test verdicts. Native-free demo APKs are valid on both x86_64 and ARM64.

Cleanup stops owned groups, discards the entire owned writable AVD, boots/reinstalls,
checks fresh demo preferences and old task absence, then stops/discards again. Dirty
state survives a crash; a quarantined host cannot accept another run. `device-recover`
requires a new host boot ID and unoccupied emulator ports before discarding stale
owned state. It never kills a process based on a PID from an old file. The next
attempt must still pass doctor, clean boot and reset. Do not use another operator's
AVD or user devices. Remote customer-account reset is not implemented by this fixture.

## Local Mac or Linux development

Run the emulator and worker directly on the host. Supported combinations are macOS
ARM64 + arm64-v8a image, macOS Intel + x86_64 image, and Linux x86_64 + x86_64 image.
An explicit profile/image mismatch fails before boot; there is no silent software
emulation fallback. Android API 35, screen dimensions, locale and case remain shared.

Prerequisites: Python/uv from the repository setup and JDK 17. The local wrapper uses
JAVA_HOME if supplied, otherwise discovers JDK 17 through java_home on Mac (with the
Apple Silicon Homebrew JDK location as a fallback). No global shell config is changed.

```sh
just device-local-setup
# Review/accept SDK licenses with the exact sdkmanager command printed by setup.
just device-local-build
just device-local              # expected passed + verified_clean
just device-local broken       # expected failed + verified_clean
just device-local unavailable  # expected blocked + verified_clean
```

Setup installs only the native image from Google's pinned archives into the dedicated
`.private/android-sdk` directory, creates a nonsecret `.private/device-local/profile.toml`
if absent, and preserves existing profiles. It neither accepts licenses nor runs checks.
Build assembles both demo flavors once; subsequent runs do not build or install tools.
Mac defaults to a visible phone. Set `headless = true` in the profile for background use.

Local mode uses real emulator boot, ADB/UI interaction, independent assertions, PNG/XML
evidence and clean reset. Navigation defaults to `adb-demo`: deterministic taps on only
our demo controls. It fetches no secrets and makes no model calls. Reports explicitly
name the navigation driver; this smoke result does not qualify Minitap. Model mode is:

```sh
just device-local-agent YOUR_MODEL_ID
just device-local-agent YOUR_MODEL_ID broken
```

Model mode uses the same supervisor with Minitap navigation and Doppler process injection;
configure OPENAI_API_KEY in the profile's selected Doppler project/config. Explicit model
choice is required and calls are billed. No environment file is created. Each run gets an
immutable private profile/request/evidence directory; latest.json points to its report.
The local command exits 0 only when the expected scenario outcome AND reset match,
1 for a mismatch or quarantine, and 2 for input/preflight errors.

The local command owns its fixture backend and disposable emulator. An occupied fixture
port or emulator port fails; it never borrows a running user's phone. Ctrl-C invokes the
same bounded cleanup/reset. After a hard crash/quarantine, reboot the host and run
`uv run --no-sync --project apps/mobile-worker --frozen mobile-qa-worker device-recover
--profile .private/device-local/profile.toml` (one command), then retry the local smoke.
Mac recovery uses kern.bootsessionuuid, Linux uses the kernel boot_id; neither falls
back to a synthetic identity. A completed **ADB-only** attempt can instead be recovered
without reboot by passing its immutable per-attempt profile to `device-recover --profile`
and its result.json to `--local-result`. The runner checks matching attempt/profile,
ADB-only navigation, no model usage and unoccupied emulator ports before discarding its
state. This exception does not apply to Minitap runs or crashes without a final result.
Do not manually remove a dirty marker to bypass recovery.

API address and device profile are separate concerns. These commands are standalone:
they do not talk to the local or cloud API. HTTP job claims and result upload remain
phase 04. The same runner can qualify a Linux cloud host using its Linux profile and
the full Minitap campaign below; local ADB smoke is not the cloud qualification gate.

## Cloud host preparation

The application hosting direction is [Railway first](hosting.md). This harness needs
a separately qualified KVM host; do not deploy it as an ordinary Railway service and
assume the emulator can boot. AWS sizing below is an optional device-host candidate,
not a requirement to host the application on AWS now.

Use a dedicated Ubuntu 24.04 x86_64 host/user with KVM permissions, JDK 17, Python 3.12,
uv 0.12.1 and Doppler. Candidate size is M8i.xlarge (4 vCPU/16 GiB), not a performance
claim. Region/AMI/access/rate/model/spend are explicit operator inputs. No script
creates paid resources. `infra/device-host/launch.example.json` is a review template,
not a runnable AWS request: remove its review marker and fill approved AMI/network/IAM
settings. Enable nested virtualization, encrypted disk and IMDSv2; expose no ADB,
emulator or fixture-backend ports. Use scoped SSM access or restricted operator SSH.

The pinned Android archive metadata lives in `infra/device-host/toolchain.lock.json`.
It records official Google archive URLs, vendor SHA1 checksums, package metadata and
revisions; our evidence uses SHA256. Archives are fetched explicitly, never during run.
The installer refuses overwriting unmanaged packages. Preserve required SDK licenses;
installation does not accept them on the user's behalf. After reviewing terms, run
sdkmanager `--licenses` explicitly with the dedicated SDK root. Tool use remains subject to the operator/organization's Android SDK terms; the
installer records package metadata without creating acceptance files. The demo-only
installer also supplies pinned platform tools so Gradle does not fetch missing tools.

```sh
bash infra/device-host/setup.sh /opt/mobile-qa/android-sdk
uv sync --project apps/mobile-worker --frozen --extra sdk
# Select only the dedicated service user's Doppler project/config using existing auth.
# Copy profile.toml to a private location; fill actual absolute paths and approved model.
just device-doctor /var/lib/mobile-qa/profile.toml
```

The tracked profile is a template and rejects its placeholder model. Minitap 4.0.0
uses an explicit OpenAI model/profile; `OPENAI_API_KEY` is injected into its child by
Doppler. The child removes unrelated secrets/tracing exports, disables dotenv discovery
and telemetry before imports, and runs in a new private cwd without an env file.
The emulator and fixture backend do not receive fetched model credentials. Use a
restricted service credential/config, not a production application's secret set.

The host service is deliberately not enabled at boot and does not restart a failed
campaign. `KillMode=control-group` contains descendants; RuntimeMaxSec and stop timeouts
bound host process work. Service timeout is not instance billing shutdown: configure
an approved instance shutdown deadline separately and verify stop/termination manually.

## Build and run

The isolated Java/Views fixture uses AGP 8.13.0, Gradle 8.13, JDK 17 and Android 35. Good
commits a task to SharedPreferences; broken keeps it only in memory. Both use identical
package/UI. A loopback HTTP session fixture deliberately returns 200 or 503; the emulator
reaches it through an explicit ADB reverse tunnel at 127.0.0.1:8765. The runner creates
the tunnel after installation, avoiding cold-boot Wi-Fi initialization races. No real
account credentials are required.

The `sample` flavor is an offline input demo: enter a task name, tap Save and see
the saved text survive an app restart. It keeps package `ai.mobileqa.demo` and the
same view IDs, but skips the controlled session prerequisite. Build it with
`:app:assembleSampleDebug`; the output is
`apps/qa-demo-android/app/build/outputs/apk/sample/debug/app-sample-debug.apk`.
This is for trying APK upload and simple input checks; it is not the good/broken
qualification fixture and does not demonstrate backend-outage handling.

For the dashboard's Create app form use name **QA Tasks sample**, package
`ai.mobileqa.demo` and environment **Local demo**. The current form requires a
backend origin, so use the reserved local fixture origin `http://127.0.0.1:8765`;
this offline APK makes no request to it. Leave login origins empty.

```sh
# Explicit SDK/dependency preparation, then build once after complete edits.
python3 infra/device-host/install_tools.py --sdk-root /opt/mobile-qa/android-sdk --demo-only
# Set ANDROID_HOME and JAVA_HOME in this process as appropriate for the dedicated host.
cd apps/qa-demo-android
./gradlew --no-daemon :app:assembleGoodDebug :app:assembleBrokenDebug :app:lintGoodDebug :app:lintBrokenDebug
```

Campaign TOML (nonsecret, stored outside Git under `.private/` or the host state root):

```toml
good_apk = "/opt/mobile-qa/repo/apps/qa-demo-android/app/build/outputs/apk/good/debug/app-good-debug.apk"
broken_apk = "/opt/mobile-qa/repo/apps/qa-demo-android/app/build/outputs/apk/broken/debug/app-broken-debug.apk"
profile = "/var/lib/mobile-qa/profile.toml"
output_root = "/var/lib/mobile-qa/evidence"
max_seconds = 14400
budget_usd = "10"
# budget is an example to replace with approved spend, never spending authorization.
# Optional host_hourly_usd/input_usd_per_million/output_usd_per_million are decimal strings.
# Include rate_source and rate_date when supplying a current quote.
```

Run from repo root with `just device-qualify /var/lib/mobile-qa/campaign.toml`, or install
and explicitly start the reviewed systemd unit. For one attempt use
`just device-smoke /absolute/request.json`; the Rust QualificationRequest schema defines
that JSON. Start the controlled fixture backend first for single-run use.

Single-run exit 0 means a terminal result was persisted, including failed/blocked/
inconclusive. Exit 2 means invalid input/setup or no persistable result. Campaign exit 0
means all 11 expected outcomes and cleanup checks succeeded, exit 1 qualification rejected,
exit 2 setup failure. Never retry uncertain side effects automatically.

## Evidence, limits and cost

Each completed attempt has result.json, a readable report, required PNG/XML, safe phase events,
logs with file/attempt limits, SDK traces when produced and optional video (up to 180 seconds). Missing
video is explicit; screenshots/XML are required for a business verdict. Log/trace data
is private diagnostic material. Do not publish it as customer-facing model reasoning.
Primary artifact paths/checksums/sizes are in the result. A failed cleanup preserves the
business observation but marks quarantined and rejects the campaign.

Timeouts/step limits and evidence/disk limits are explicit in the profile. SDK callbacks
record observed usage, deduplicating callback IDs; unknown usage is not zero. Monetary
estimates may be partial, and already in-flight model requests can incur charges.
A time limit and a soft estimated-cost threshold are not a provider-side spending cap.
Use an approved model-provider cap and host shutdown deadline for a strict total limit.
Campaign cost excludes storage/IP/transfer, host time outside the campaign and human
review; add these to the qualification record from actual billing/measurement.

Before terminating the only evidence host, copy its private evidence tree through the
approved operator channel, verify available-artifact hashes against result.json, and
record transfer start/end, bytes and duration in the qualification record. Keep a stopped
encrypted disk if transfer fails. No production artifact storage/upload pipeline exists
in this phase. Never label a cloud run qualified from offline/mocked tests alone.

## References

- [AWS nested virtualization](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/amazon-ec2-nested-virtualization.html)
- [Android acceleration](https://developer.android.com/studio/run/emulator-acceleration)
- [Android emulator commands](https://developer.android.com/studio/run/emulator-commandline)
- [ADB evidence/control](https://developer.android.com/tools/adb)
- [Minitap SDK installation](https://www.minitap.ai/docs/mobile-use-sdk/installation)
- [AGP compatibility](https://developer.android.com/build/releases/agp-8-13-0-release-notes)
