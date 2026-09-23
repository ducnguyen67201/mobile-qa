#!/bin/sh
# Build-time tools only; pins are official GitHub release asset SHA-256 values.
set -eu
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
  https://github.com/DopplerHQ/cli/releases/download/3.76.6/doppler_3.76.6_linux_amd64.tar.gz \
  --output "$work/doppler.tar.gz"
printf '%s  %s\n' 67e4e020761adf3ffe5a030712d61721b4e2752182670bf90de5a2a88e4961e3 "$work/doppler.tar.gz" | sha256sum --check --status
tar -xzf "$work/doppler.tar.gz" -C "$work" doppler
install -m 0755 "$work/doppler" /usr/local/bin/doppler
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
  https://github.com/astral-sh/uv/releases/download/0.12.17/uv-x86_64-unknown-linux-gnu.tar.gz \
  --output "$work/uv.tar.gz"
printf '%s  %s\n' fa82fd8dde8e8eefdecada6aa0889666556cfceb690d06e0c3bca49eb3070a63 "$work/uv.tar.gz" | sha256sum --check --status
tar -xzf "$work/uv.tar.gz" -C "$work"
install -m 0755 "$work/uv-x86_64-unknown-linux-gnu/uv" /usr/local/bin/uv
