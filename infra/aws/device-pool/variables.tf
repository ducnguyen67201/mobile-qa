variable "region" { type = string }
variable "availability_zone" { type = string }
variable "pool_id" {
  type = string
  validation {
    condition     = can(regex("^[0-9a-f-]{36}$", var.pool_id))
    error_message = "pool_id must be the operator-registered UUID."
  }
}
variable "name" {
  type    = string
  default = "mobile-qa-pilot"
  validation {
    condition     = can(regex("^[a-z][a-z0-9-]{2,31}$", var.name))
    error_message = "Use 3–32 lowercase letters, digits or hyphens."
  }
}
variable "ami_id" {
  type = string
  validation {
    condition     = can(regex("^ami-[0-9a-f]+$", var.ami_id))
    error_message = "Use an explicitly approved immutable AMI ID."
  }
}
variable "instance_type" {
  type = string
  validation {
    condition     = contains(["m7i.large", "m7i.xlarge", "m7i.2xlarge", "m8i.large", "m8i.xlarge", "m8i.2xlarge"], var.instance_type)
    error_message = "Select a nested-virtualization candidate and separately qualify its slot count."
  }
}
variable "api_origin" {
  type = string
  validation {
    condition     = can(regex("^https://[A-Za-z0-9.-]+(:[0-9]+)?$", var.api_origin))
    error_message = "API origin must be HTTPS without path, credentials or trailing slash."
  }
}
variable "host_id" { type = string }
variable "host_slots" {
  type = list(object({
    index      = number, app_id = string, profile_id = string,
    memory_mib = optional(number, 4096), cores = optional(number, 2),
    rendering  = optional(string, "software"), warm_qualification = optional(string, "")
  }))
  validation {
    condition     = length(var.host_slots) >= 1 && length(var.host_slots) <= 2 && length(distinct([for slot in var.host_slots : slot.index])) == length(var.host_slots) && alltrue([for slot in var.host_slots : contains([0, 1], slot.index)])
    error_message = "The pilot has one or two explicitly qualified slots with unique indexes 0 or 1."
  }
  validation {
    condition     = alltrue([for slot in var.host_slots : slot.memory_mib >= 1024 && slot.memory_mib <= 16384 && slot.memory_mib == floor(slot.memory_mib) && slot.cores >= 1 && slot.cores <= 8 && slot.cores == floor(slot.cores) && contains(["software", "swiftshader"], slot.rendering)])
    error_message = "Slot resources must be integer MiB/cores and rendering software or swiftshader."
  }
}
variable "host_bootstrap_secret_arn" {
  type        = string
  description = "Existing Secrets Manager secret containing only the host's scoped Doppler service token. No secret version is managed here."
}
variable "controller_bootstrap_secret_arn" {
  type        = string
  description = "Existing Secrets Manager secret containing only the controller's separate Doppler service token."
}
variable "bootstrap_kms_key_arns" {
  type        = list(string)
  default     = []
  description = "Only customer-managed KMS keys needed to decrypt the two bootstrap secrets."
}
variable "doppler_project" { type = string }
variable "host_doppler_config" { type = string }
variable "controller_doppler_config" { type = string }
variable "controller_image_uri" {
  type = string
  validation {
    condition     = can(regex("^[0-9]+\\.dkr\\.ecr\\.[a-z0-9-]+\\.amazonaws\\.com/[a-z0-9/_-]+@sha256:[0-9a-f]{64}$", var.controller_image_uri))
    error_message = "Supply a reviewed ECR image digest, never a mutable tag."
  }
}
variable "controller_enabled" {
  type        = bool
  default     = false
  description = "Enable only after API registration, host qualification and drain acceptance. Disabled also sets Lambda concurrency to zero."
}
variable "log_retention_days" {
  type    = number
  default = 14
}
variable "alarm_topic_arn" {
  type        = string
  description = "Existing SNS topic for capacity errors/throttles; notification ownership remains outside this module."
}
variable "network_isolation_evidence" {
  type        = string
  default     = ""
  description = "Operator-reviewed immutable evidence reference proving guest denial of IMDS, host loopback and private networks for this exact AMI. The checked-in cgroup/nft launcher must pass live Linux packet tests before this assertion is supplied."
}

variable "host_warm_schedule_enabled" {
  type        = bool
  default     = false
  description = "Extra local warm gate; API windows, operator evidence and server slot qualification must also permit preboot."
}
