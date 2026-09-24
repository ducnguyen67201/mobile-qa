packer {
  required_plugins {
    amazon = {
      version = "= 1.8.2"
      source  = "github.com/hashicorp/amazon"
    }
  }
}
variable "region" { type = string }
variable "source_ami" { type = string }
variable "builder_subnet_id" { type = string }
variable "builder_security_group_id" { type = string }
variable "release_archive" { type = string }
variable "release_sha256" { type = string }
variable "git_revision" { type = string }
variable "sdk_licenses_accepted" {
  type    = bool
  default = false
}
source "amazon-ebs" "device" {
  region            = var.region
  source_ami        = var.source_ami
  instance_type     = "m7i.large"
  subnet_id         = var.builder_subnet_id
  security_group_id = var.builder_security_group_id
  ssh_username      = "ubuntu"
  ami_name          = "mobile-qa-device-${substr(var.git_revision, 0, 12)}-${formatdate("YYYYMMDDhhmmss", timestamp())}"
  encrypt_boot      = true
  launch_block_device_mappings {
    device_name           = "/dev/sda1"
    volume_size           = 100
    volume_type           = "gp3"
    delete_on_termination = true
    encrypted             = true
  }
  metadata_options {
    http_endpoint = "enabled"
    http_tokens   = "required"
  }
  tags = { Project = "mobile-qa", GitRevision = var.git_revision, Qualification = "required" }
}
build {
  sources = ["source.amazon-ebs.device"]
  provisioner "file" {
    source      = var.release_archive
    destination = "/tmp/mobile-qa-release.tar"
  }
  provisioner "shell" {
    script = "${path.root}/provision.sh"
    environment_vars = [
      "RELEASE_SHA256=${var.release_sha256}",
      "GIT_REVISION=${var.git_revision}",
      "SDK_LICENSES_ACCEPTED=${var.sdk_licenses_accepted}"
    ]
    execute_command = "sudo -E env {{ .Vars }} bash '{{ .Path }}'"
  }
  post-processor "manifest" { output = "manifest.json" }
}
