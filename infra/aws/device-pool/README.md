# Fixed pilot device host

These are deployment definitions, not a deployed or qualified device farm. Read the
canonical architecture and environment documents before release. One EC2 instance
is declared permanently; the capacity controller owns only its running/stopped
state. There is no autoscaling group, NAT gateway, Kubernetes or Redis.

## Local validation

Use OpenTofu **1.12.6** and Packer **1.16.1**. The provider lock pins AWS **6.66.0**
for Linux amd64 and macOS arm64. The Packer Amazon plugin is **1.8.2**.

```sh
python3 scripts/check_deployment.py
python3 -m unittest scripts.test_check_deployment
uv sync --project infra/aws/capacity-controller --frozen
uv run --project infra/aws/capacity-controller --frozen ruff check infra/aws/capacity-controller
(cd infra/aws/capacity-controller && uv run --frozen pyright && uv run --frozen pytest)
```

The infrastructure checker downloads public provider/plugin packages, then validates
configuration with cloud credentials removed and state backend disabled. It never
runs `plan`, `apply`, Packer build, an emulator or a model. Generated worker contracts
must already be current before controller checks.

## Local image builds

Build only after source generation/checks complete. These commands keep images local;
a release push/deployment is a separate authorized action. Lambda requires a single
architecture image, so disable BuildKit provenance attestations for its image:

```sh
docker build --platform linux/amd64 --file Dockerfile.api --tag mobile-qa-api:local .
docker build --platform linux/amd64 --provenance=false --file infra/aws/capacity-controller/Dockerfile --tag mobile-qa-capacity:local .
```

No runtime token, provider credential or APK is a build argument. Image startup is
not part of an ordinary image build; live bootstrap requires the registered secret
references. The Lambda image's generated contracts are copied from this checkout.

## State and inputs

Use a separately bootstrapped private S3 backend with encryption, versioning, public
access blocked and lock support enabled (`use_lockfile = true`). Restrict its IAM
read/write permissions to deployment operators. Supply backend bucket, key and
region through reviewed nonsecret backend configuration. Never supply access keys
in backend arguments or variable files. State can contain infrastructure identity
and nonsecret host configuration; it never reads Secrets Manager secret values.

Supply an explicit region, AZ, approved AMI, supported instance type, pool/host IDs,
slot app/profile bindings, API HTTPS origin, ECR controller image **digest**, alarm
SNS topic and the two pre-existing bootstrap secret **ARNs**. The host secret stores
only its scoped Doppler service token. The separate controller secret stores only
its controller-scoped Doppler token. If those secrets use customer-managed KMS keys,
list only those keys; decryption is restricted to Secrets Manager. Scoped application
credentials live in Doppler, with hashes stored by operator API registration.

`network_isolation_evidence` is empty by default, so EC2 provisioning fails closed.
It must reference reviewed acceptance evidence for the exact image/network boundary.
A filled string is an operator assertion, **not a substitute for implementing and
testing isolation**. Linux guest network rules, root launcher, metadata denial,
private/loopback denial, IPv6, TCP/UDP/DNS rebinding, ADB and traffic after reboot must
all pass the hosted gate. Security groups and IMDS hop limits do not isolate the
Android guest from its host.

The controller is disabled by default (`controller_enabled = false`): its schedule
is disabled and reserved concurrency is zero. An authorized first apply **still
launches the fixed EC2 instance** for commissioning; budget this boot and stop it
after commissioning. This module does not manage an `aws_ec2_instance_state`
resource. Never disable the instance through a resource count or destroy operation.

## Network and persistence

The host has no inbound security-group rules and no SSH key. Operators use SSM with
separate operator IAM authority. Outbound TCP 443 reaches API, storage, Doppler,
model providers and application backends. TCP/UDP 53 is permitted only to
1.1.1.1 and 8.8.8.8, matching Android's explicit DNS configuration. HTTP-only customer backends do not work
under this policy; adding egress is a reviewed qualification change. A public subnet
and ephemeral public IPv4 avoid a NAT gateway. Public IPv4 can incur charges while
assigned; it can change after stop/start. Host identity uses the instance ID, not IP.

The 100 GiB gp3 root disk is encrypted, retained on stop and retained on termination.
Termination protection plus `prevent_destroy` makes image/user-data replacement an
explicit migration. Retained old disks continue to cost money until deliberately
purged. The initial cache cap is 50 GiB; free-space admission protects Android,
installation, scratch and evidence headroom. Neither API/storage costs nor retained
EBS charges stop when EC2 stops.

## Commissioning order (requires deployment authorization)

1. Commit and review release source. Bake the AMI from `git archive` using the AMI
   runbook. Build API and controller images for Linux amd64; scan and record their
   immutable digests. No runtime secret is passed into a build.
2. On a separately authorized, isolated staging candidate using that exact AMI, run
   the live network/kernel campaign before the fixed production pool can be created.
   Record real evidence; never use a placeholder reference to bypass the precondition.
3. Deploy additive schema/API compatibility with pool and new upload features paused.
   Provision bootstrap secret references outside OpenTofu. Register the pool, host,
   approved slots/profiles and token hashes through the trusted API operator CLI.
4. Review an authorized OpenTofu plan against protected remote state. Check region,
   public IP/EBS costs, IAM scope and all replacement actions. Apply once approved.
   Reconcile the API's allowed instance ID with the actual output before enabling.
5. Verify systemd, image/package hashes, KVM, secret bootstrap, guest isolation,
   registration, app/model qualification and one cold slot. Capture the evidence
   reference; fake tests and source inspection do not prove Linux kernel behavior.
6. Test stop/start, cache retention, boot-generation fencing, lost AWS response,
   duplicate hints, demand before/after stop commit, cleanup failure and API outage.
7. Enable the controller last. `warm_target` remains zero until independent warm
   reset/ownership qualification. Raise slot count only after the measured campaign.

## Operation and incident boundaries

A wake hint is HMAC authenticated, expires after 60 seconds and is durably consumed
once by the API. It identifies a pool; it cannot select an arbitrary EC2 instance.
The controller always reads current demand, verifies the fixed instance ID and
records an API operation before calling AWS. Every minute it reconciles missed hints.

A stop needs API drain plus fresh host cleanup. During ambiguous AWS responses the
controller never repeats an old write: it polls the recorded operation's outcome.
After 15 minutes unresolved state is quarantined and a Lambda error triggers the
configured alarm. This may require operator intervention even if the AWS request
was never issued; it avoids a delayed stop against a later boot. API outage never
authorizes stopping. SDK write retries are disabled and Lambda concurrency is one.

Pausing pool admission and draining is the normal rollback. Disable the controller
only after observing any committed operation; otherwise it can leave compute running.
Emergency spending containment requires an operator-fenced stop: preserve job/device
reservations and quarantined journals, record observed power, and reconcile before
accepting any new generation. A force stop does not assert clean job completion.
Do not delete recovery journals, overwrite token hashes or run `tofu destroy` to idle.

Rotate a bootstrap token in the provider secret store, then restart its service or
publish/update the Lambda release to force fresh bootstrap after clean drain. Doppler
runtime secret refresh follows process startup; no secret file fallback is used.
An idle warm Lambda does not automatically reload a changed bootstrap token.

Provider references: [nested virtualization](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/amazon-ec2-nested-virtualization.html),
[stop/start persistence](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/Stop_Start.html),
[Lambda container requirements](https://docs.aws.amazon.com/lambda/latest/dg/images-create.html).
