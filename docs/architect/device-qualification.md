# Android runner qualification

Phase 02 is an operator experiment against our controlled demo APK. It is not the
phase 04 HTTP worker lease protocol, and passing this experiment does not establish
customer-app support or production reliability. The existing fake/import commands
remain separate. Implementation/validation status is recorded in [status](status.md).

## What the harness does

`device-doctor` reads a profile and checks Linux x86_64, usable KVM, pinned package
revisions and disk. `device-run` owns one emulator-5554 attempt; `device-qualify` runs
three good/broken/backend503 cycles, an interruption probe after navigation starts,
and one recovery run. The campaign oracle never enters the worker request or prompt.

Minitap enters and saves a unique task. Our code observes its exact package-scoped
row, force-stops/reopens the app and checks persistence. Missing evidence or failed
navigation is inconclusive. Backend unavailability is blocked. A verified saved task
that disappears is failed. SDK completion and trace `_PASS` suffixes are diagnostic,
not test verdicts. Native-free demo APKs are valid on x86_64.

Cleanup stops owned groups, discards the entire owned writable AVD, boots/reinstalls,
checks fresh demo preferences and old task absence, then stops/discards again. Dirty
state survives a crash; a quarantined host cannot accept another run. `device-recover`
requires a new host boot ID and unoccupied emulator ports before discarding stale
owned state. It never kills a process based on a PID from an old file. The next
attempt must still pass doctor, clean boot and reset. Do not use another operator's
AVD or user devices. Remote customer-account reset is not implemented by this fixture.

## Host preparation

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
reaches it through 10.0.2.2. No real account credentials are required.

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
