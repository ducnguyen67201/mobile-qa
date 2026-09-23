"""No network or tools: verify credential removal and the provider schema gate."""

import os
import unittest
from unittest.mock import patch

from scripts.check_deployment import (
    assert_nested_virtualization,
    offline_environment,
    provider_schema_configuration,
)


class DeploymentCheckTests(unittest.TestCase):
    def test_checks_do_not_inherit_cloud_or_runtime_credentials(self):
        with patch.dict(
            os.environ,
            {
                "AWS_PROFILE": "production",
                "AWS_ACCESS_KEY_ID": "secret",
                "DOPPLER_TOKEN": "secret",
                "TF_VAR_token": "secret",
                "PKR_VAR_token": "secret",
                "PATH": "/usr/bin",
            },
            clear=True,
        ):
            result = offline_environment()
        self.assertNotIn("AWS_PROFILE", result)
        self.assertNotIn("AWS_ACCESS_KEY_ID", result)
        self.assertNotIn("DOPPLER_TOKEN", result)
        self.assertNotIn("TF_VAR_token", result)
        self.assertNotIn("PKR_VAR_token", result)
        self.assertEqual(result["AWS_SHARED_CREDENTIALS_FILE"], os.devnull)
        self.assertEqual(result["AWS_EC2_METADATA_DISABLED"], "true")

    def test_schema_configuration_excludes_remote_backend_and_provider_settings(self):
        source = """terraform {
  required_providers { aws = { source = "registry.opentofu.org/hashicorp/aws", version = "= 6.66.0" } }
  backend "s3" {}
}
provider "aws" { region = var.region }
"""
        isolated = provider_schema_configuration(source)
        self.assertIn('version = "= 6.66.0"', isolated)
        self.assertNotIn('backend "s3"', isolated)
        self.assertNotIn("var.region", isolated)
        with self.assertRaises(ValueError):
            provider_schema_configuration(
                source.replace(
                    'backend "s3" {}', 'backend "s3" { bucket = "production" }'
                )
            )

    def test_provider_without_nested_virtualization_is_rejected(self):
        attributes = {}
        schema = {
            "provider_schemas": {
                "registry.terraform.io/hashicorp/aws": {
                    "resource_schemas": {
                        "aws_instance": {
                            "block": {
                                "block_types": {
                                    "cpu_options": {"block": {"attributes": attributes}}
                                }
                            }
                        }
                    }
                }
            }
        }
        with self.assertRaises(ValueError):
            assert_nested_virtualization(schema)
        attributes["nested_virtualization"] = {"type": "string"}
        assert_nested_virtualization(schema)
        providers = schema["provider_schemas"]
        providers["registry.opentofu.org/hashicorp/aws"] = providers.pop(
            "registry.terraform.io/hashicorp/aws"
        )
        assert_nested_virtualization(schema)


if __name__ == "__main__":
    unittest.main()
