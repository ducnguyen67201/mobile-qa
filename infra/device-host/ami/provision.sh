#!/bin/bash
# Explicit paid AMI build only. Never invoked by ordinary checks or EC2 boot.
set -euo pipefail
[[ "$SDK_LICENSES_ACCEPTED" == true ]] || { echo 'Review and accept Android SDK licenses before building.' >&2; exit 2; }
[[ "$RELEASE_SHA256" =~ ^[a-f0-9]{64}$ && "$GIT_REVISION" =~ ^[a-f0-9]{40}$ ]]
printf '%s  %s\n' "$RELEASE_SHA256" /tmp/mobile-qa-release.tar | sha256sum --check --status
source /etc/os-release
[[ "$ID" == ubuntu && "$VERSION_ID" == 24.04 ]] || { echo 'Approved base must be Ubuntu 24.04.' >&2; exit 2; }
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install --yes --no-install-recommends ca-certificates curl python3 openjdk-17-jre-headless \
  libpulse0 libx11-6 libxcb1 libxdamage1 libnss3 libxcomposite1 libxcursor1 libxi6 libxrandr2 \
  libxtst6 libglib2.0-0 libasound2t64 libgl1 libdbus-1-3 unzip qemu-kvm gcc libc6-dev nftables iproute2
getent group kvm >/dev/null || groupadd --system kvm
useradd --system --create-home --home-dir /var/lib/mobile-qa --groups kvm mobile-qa
install -d -m 0755 /opt/mobile-qa/repo /opt/mobile-qa/bootstrap
# The release tar is made by git archive. It contains no .git, ignored private
# state, credentials or customer APKs; the approved SHA binds these exact bytes.
tar -xf /tmp/mobile-qa-release.tar -C /opt/mobile-qa/repo --no-same-owner
cd /opt/mobile-qa/repo
sh infra/deployment/install_runtime.sh
export UV_PYTHON_INSTALL_DIR=/opt/mobile-qa/python
uv sync --project apps/mobile-worker --frozen --no-dev --extra device --extra ai --extra sdk --python 3.12
UV_PROJECT_ENVIRONMENT=/opt/mobile-qa/bootstrap/.venv uv sync --project infra/aws/capacity-controller --frozen --no-dev --python 3.12
python3 infra/device-host/install_tools.py --sdk-root /opt/mobile-qa/android-sdk
# Acceptance is a required, explicit build input. Only public SDK license hashes
# are persisted; there are no Doppler or cloud credentials in the image.
set +o pipefail
yes | JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64 /opt/mobile-qa/android-sdk/cmdline-tools/19.0/bin/sdkmanager --sdk_root=/opt/mobile-qa/android-sdk --licenses >/dev/null
sdk_status=${PIPESTATUS[1]}
set -o pipefail
[[ "$sdk_status" == 0 ]]
bash infra/device-host/network/install.sh
install -m 0644 infra/device-host/mobile-qa-host.service /etc/systemd/system/mobile-qa-host.service
systemctl enable mobile-qa-host.service
python3 infra/device-host/ami/verify_image.py --write "$GIT_REVISION" "$RELEASE_SHA256"
chmod -R a+rX,go-w /opt/mobile-qa
chown -R root:root /opt/mobile-qa
rm -f /tmp/mobile-qa-release.tar
apt-get clean
rm -rf /var/lib/apt/lists/* /root/.cache/uv /root/.android
# Fail the bake if the approved source AMI lacks the SSM agent.
systemctl cat amazon-ssm-agent.service >/dev/null 2>&1 || systemctl cat snap.amazon-ssm-agent.amazon-ssm-agent.service >/dev/null
