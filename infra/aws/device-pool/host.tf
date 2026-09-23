resource "aws_vpc" "pool" {
  cidr_block           = "10.76.0.0/24"
  enable_dns_support   = true
  enable_dns_hostnames = true
}
resource "aws_internet_gateway" "pool" { vpc_id = aws_vpc.pool.id }
resource "aws_subnet" "host" {
  vpc_id            = aws_vpc.pool.id
  cidr_block        = "10.76.0.0/26"
  availability_zone = var.availability_zone
}
resource "aws_route_table" "host" {
  vpc_id = aws_vpc.pool.id
  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.pool.id
  }
}
resource "aws_route_table_association" "host" {
  subnet_id      = aws_subnet.host.id
  route_table_id = aws_route_table.host.id
}
resource "aws_security_group" "host" {
  name_prefix = "${var.name}-host-"
  description = "No inbound ports. HTTPS to API, model providers, SSM and artifact storage."
  vpc_id      = aws_vpc.pool.id
}
resource "aws_vpc_security_group_egress_rule" "https" {
  security_group_id = aws_security_group.host.id
  cidr_ipv4         = "0.0.0.0/0"
  ip_protocol       = "tcp"
  from_port         = 443
  to_port           = 443
}

# Android user-mode networking must not forward DNS through host/private resolvers.
# These match the emulator's explicit DNS configuration; destination IP filtering
# remains enforced after resolution to reject rebinding to private/metadata hosts.
resource "aws_vpc_security_group_egress_rule" "dns" {
  for_each = {
    cloudflare_udp = { protocol = "udp", address = "1.1.1.1/32" }
    cloudflare_tcp = { protocol = "tcp", address = "1.1.1.1/32" }
    google_udp     = { protocol = "udp", address = "8.8.8.8/32" }
    google_tcp     = { protocol = "tcp", address = "8.8.8.8/32" }
  }
  security_group_id = aws_security_group.host.id
  cidr_ipv4         = each.value.address
  ip_protocol       = each.value.protocol
  from_port         = 53
  to_port           = 53
}

locals {
  host_config = join("\n", concat([
    "origin = ${jsonencode(var.api_origin)}",
    "host_id = ${jsonencode(var.host_id)}",
    "state_root = \"/var/lib/mobile-qa/host\"",
    "profile = \"/etc/mobile-qa/profile.toml\"",
    "warm_schedule = ${var.host_warm_schedule_enabled}"
    ], flatten([for slot in var.host_slots : [
      "[[slots]]", "index = ${slot.index}",
      "app_id = ${jsonencode(slot.app_id)}", "profile_id = ${jsonencode(slot.profile_id)}",
      "memory_mib = ${slot.memory_mib}", "cores = ${slot.cores}",
      "rendering = ${jsonencode(slot.rendering)}", "warm_qualification = ${jsonencode(slot.warm_qualification)}"
  ]])))
  device_profile = join("\n", [
    "sdk_root = \"/opt/mobile-qa/android-sdk\"",
    "state_root = \"/var/lib/mobile-qa/device\"",
    "toolchain = \"/opt/mobile-qa/repo/infra/device-host/toolchain.lock.json\"",
    "headless = true",
    "install_seconds = 600",
    "disk_min_bytes = 16106127360",
    "doppler_project = ${jsonencode(var.doppler_project)}",
    "doppler_config = ${jsonencode(var.host_doppler_config)}"
  ])
  bootstrap_config = jsonencode({
    secret_arn = var.host_bootstrap_secret_arn, region = var.region,
    project    = var.doppler_project, config = var.host_doppler_config
  })
}

resource "aws_instance" "host" {
  ami                                  = var.ami_id
  instance_type                        = var.instance_type
  subnet_id                            = aws_subnet.host.id
  associate_public_ip_address          = true
  vpc_security_group_ids               = [aws_security_group.host.id]
  iam_instance_profile                 = aws_iam_instance_profile.host.name
  disable_api_termination              = true
  instance_initiated_shutdown_behavior = "stop"
  user_data_replace_on_change          = true
  user_data = templatefile("${path.module}/user-data.sh.tftpl", {
    host_config      = base64encode(local.host_config),
    device_profile   = base64encode(local.device_profile),
    bootstrap_config = base64encode(local.bootstrap_config)
  })
  cpu_options { nested_virtualization = "enabled" }
  metadata_options {
    http_endpoint               = "enabled"
    http_tokens                 = "required"
    http_put_response_hop_limit = 1
  }
  root_block_device {
    encrypted             = true
    volume_type           = "gp3"
    volume_size           = 100
    delete_on_termination = false
  }
  tags = { Name = "${var.name}-device" }
  lifecycle {
    # Power is controlled by Lambda. AMI/user-data replacement needs a reviewed
    # migration and deliberate removal of this guard, never an idle scaling action.
    prevent_destroy = true
    precondition {
      condition     = sum([for slot in var.host_slots : slot.cores]) <= (endswith(var.instance_type, ".large") ? 2 : endswith(var.instance_type, ".xlarge") ? 4 : 8) && sum([for slot in var.host_slots : slot.memory_mib]) <= (endswith(var.instance_type, ".large") ? 6144 : endswith(var.instance_type, ".xlarge") ? 14336 : 30720)
      error_message = "Slot allocations must fit the host CPU and retain at least 2 GiB for the host. Qualification may require more headroom."
    }
    precondition {
      condition     = length(var.network_isolation_evidence) > 0
      error_message = "Device deployment is blocked until actual guest-network isolation is implemented and qualified. Security groups and IMDS hop limits do not isolate QEMU guest traffic."
    }
  }
}
