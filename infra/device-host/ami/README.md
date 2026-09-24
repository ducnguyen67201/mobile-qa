# Device AMI release

Packer builds launch paid EC2 resources. This directory is configuration only;
ordinary checks use `packer validate` and never run the build.

The approved source AMI must be **Ubuntu 24.04 x86_64** with its SSM agent installed.
Supply a fixed source AMI ID, region, dedicated builder subnet/security group, release
archive SHA-256, 40-character Git revision and absolute archive path. The builder
network must permit controlled SSH from the build operator and HTTPS package access;
it is separate from the runtime host security group, which has no inbound ports.

Create the archive from an approved commit with `git archive --format=tar`. Do not
upload a working-directory tar: ignored secrets, APKs and recovery state must never
enter an AMI. `sdk_licenses_accepted` defaults to false; the build fails unless the
operator has reviewed the Android SDK licenses and explicitly records acceptance.
No deployment command here changes the user's local SDK acceptance or Doppler config.

The image contains the locked worker/runtime dependencies, pinned Android SDK, host
supervisor service and the network isolation launcher/policy. Python's managed
runtime is retained under `/opt/mobile-qa/python`; no executable depends on a deleted
builder cache. SDK and release source are root-owned and read-only to the device
user. The image manifest records release/lock hashes and the exact installed OS
package inventory. OS apt packages are resolved at bake time, so a subsequent rebuild
requires a new manifest and qualification even from the same source commit.

EC2 user data writes only nonsecret TOML and bootstrap-secret ARN configuration.
It enables the host service; subsequent stop/start boots run the enabled systemd
unit again. The service verifies image locks and KVM before fetching its narrowly
scoped Doppler bootstrap token into process memory. The supervisor then verifies
actual SDK versions and qualification; seeing `/dev/kvm` alone is insufficient.

The one-shot `mobile-qa-device.service` remains an operator qualification campaign.
The new `mobile-qa-host.service` is the persistent runtime owner. Never replace it
with a one-shot cloud-init command or run `sdkmanager --update` during a job.

## Hosted acceptance record

Keep the pool disabled until the following record exists for the exact release:

- Region, instance SKU, AMI ID, release/image/toolchain digests and profile revision.
- Guest network isolation evidence, including metadata/private/loopback/IPv6 denial,
  DNS rebinding and successful allowed app HTTPS/ADB traffic after reboot.
- Slot count and representative app SHA-256s, compressed and installed sizes.
- Boot/install/test/cleanup p50 and p95, CPU pressure, process RSS, available memory,
  disk headroom/IO, errors, OOMs, leaked processes and reset contamination checks.
- One standard slot on 2 vCPU/8 GiB; one then two standard slots on 4/16; one heavy
  app on 4/16. These are campaigns, not declared supported maxima.
- Large APK cache miss/hit, cancellation, simultaneous phone/execution contention,
  failed bootstrap, no secret files, clean drain, stop/start and cache retention.
- Actual billed host minutes, retained disk/IP/other charges and cost per completed
  run. Keep API and model charges separate from EC2 occupancy.

Use the existing explicit qualification CLI for physical/device/model exercises and
capture metrics externally during the approved campaign. This change does not claim
to have run a paid campaign or establish a universal emulator density.
