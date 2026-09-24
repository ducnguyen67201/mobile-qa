# Emulator guest network boundary

This implements the Linux network control; deployment still requires a recorded
packet-level qualification on the exact AMI, SDK and instance configuration.

## Enforcement

`mobile-qa-network.service` creates a root-owned, nondelegated cgroup v2 named
`mobile-qa-emulators` and installs one private nftables table. It runs before the
host supervisor. The root-owned ready marker binds successful installation to the
current Linux boot; absence, stale boot, unsupported kernel expression or metadata
lookup failure prevents the production launcher from starting Android.

A small setuid launcher accepts only the fixed, root-owned Android emulator binary.
It writes **its own PID** to the fixed cgroup, clears supplementary groups except
KVM, permanently drops all UID/GID privileges, clears the inherited environment,
sets `no_new_privs`, and executes Android with the same PID/process group. It opens
no caller-selected path while privileged and accepts no target PID or arbitrary
command. File descriptors are close-on-exec. The immutable AMI digest binds its
source/binary and SDK; the ready marker is installation evidence, not qualification.

The output rule identifies emulator TCP/UDP sockets by their cgroup ancestor.
Private, loopback, link-local, multicast, IPv4/IPv6 translation ranges, host-local
routes and the EC2 host's public IPv4 are denied. Public TCP/UDP remain available.
Only established **reply-direction** packets bypass destination denial, preserving
connections initiated by the host's ADB client. The supervisor, ADB server and
Doppler bootstrap remain outside this cgroup. Public DNS servers are explicitly
configured for Android so a host loopback resolver is unnecessary.

[Nftables documents](https://www.netfilter.org/projects/nftables/manpage.html) socket
cgroup ancestor matching, conntrack direction and FIB address classification. The
[Linux cgroup v2 interface](https://docs.kernel.org/admin-guide/cgroup-v2.html)
provides process migration and inherited membership. Both are required capabilities
checked during deployment qualification; IMDSv2 by itself is insufficient because
Android user-mode networking originates host sockets.

## Integration constraints

- AMI provisioning runs `bash infra/device-host/network/install.sh`; ordinary checks
  never execute that installer or alter host firewall state.
- The launcher is `/usr/local/libexec/mobile-qa-emulator`, mode `4750`, owner
  `root:mobile-qa`. Its only elevated capabilities are SETUID and SETGID under the
  host service bounding set. Cgroup files remain root-owned and nondelegated.
- `NoNewPrivileges`, `RestrictSUIDSGID` and `LockPersonality` are disabled on the
  supervisor because they prevent the narrow launcher transition. The launcher
  restores `no_new_privs` before emulator exec. Mount/path protections remain.
  [Systemd documents the inherited restriction](https://github.com/systemd/systemd/blob/main/man/systemd.exec.xml).
- All production emulator launches go through the wrapper. Legacy local qualification
  remains separate and cannot assert this hosted isolation control.
- Host ADB initiates connections to each emulator. Android must never be granted a
  firewall exception to initiate a connection to a host ADB server or IPC port.
- When the host service stops, its network unit kills the emulator cgroup and waits
  for it to empty. Rules remain installed until reboot, avoiding a teardown gap.
  Normal cleanup still proves the owned process groups and ports are gone.
- Customer apps requiring private network access are unsupported in this pilot.
  Do not broaden private subnet access to make a customer test pass.

## Required live qualification

From an Android guest, verify TCP/UDP denial to `10.0.2.2`, host ADB/console/IPC
listeners, RFC1918 destinations, `169.254.169.254`, `fd00:ec2::254`, all host-bound
addresses and its public IPv4. Probe numeric addresses and DNS names resolving to
these targets. Confirm public HTTPS and DNS work, ADB connects from the supervisor,
install/capture/reset work on two slots, and one guest cannot reach another slot.
Capture nft rule counters and actual emulator cgroup membership as evidence.

Also prove a missing/stale marker, failed rule installation, wrong binary path,
wrong caller UID, inherited `no_new_privs`, symlinked executable, excessive input,
and a host crash fail closed. Restart the host service and confirm the previous
cgroup is empty before a new supervisor can register. Test IPv6 and translation
prefixes even if the application normally uses IPv4. Ordinary tests exercise policy
construction and boundary logic with fakes; they do not prove kernel packet behavior.
