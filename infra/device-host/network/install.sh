#!/bin/bash
# AMI bake only. This does not activate firewall rules on the build host.
set -euo pipefail
[[ "$EUID" == 0 ]]
root_dir=$(cd -- "$(dirname -- "$0")" && pwd)
getent passwd mobile-qa >/dev/null
getent group kvm >/dev/null
install -d -m 0755 /usr/local/libexec
cc -std=c11 -O2 -Wall -Wextra -Werror -fPIE -pie -fstack-protector-strong \
  -D_FORTIFY_SOURCE=3 -Wl,-z,relro,-z,now \
  "$root_dir/launch-emulator.c" -o /usr/local/libexec/mobile-qa-emulator
chown root:mobile-qa /usr/local/libexec/mobile-qa-emulator
chmod 4750 /usr/local/libexec/mobile-qa-emulator
install -o root -g root -m 0644 "$root_dir/mobile-qa-network.service" \
  /etc/systemd/system/mobile-qa-network.service
