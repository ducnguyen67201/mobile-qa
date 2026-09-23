terraform {
  required_version = ">= 1.9.0, < 2.0.0"
  required_providers {
    aws = {
      source  = "registry.opentofu.org/hashicorp/aws"
      version = "= 6.66.0"
    }
  }
  # Supply bucket/key/region through reviewed nonsecret backend configuration.
  # Bootstrap the encrypted, versioned state bucket separately; never store credentials here.
  backend "s3" {}
}

provider "aws" {
  region = var.region
  default_tags {
    tags = { Project = "mobile-qa", Pool = var.pool_id, ManagedBy = "OpenTofu" }
  }
}
