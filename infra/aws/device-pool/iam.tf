data "aws_caller_identity" "current" {}
locals {
  ec2_trust         = jsonencode({ Version = "2012-10-17", Statement = [{ Effect = "Allow", Principal = { Service = "ec2.amazonaws.com" }, Action = "sts:AssumeRole" }] })
  lambda_trust      = jsonencode({ Version = "2012-10-17", Statement = [{ Effect = "Allow", Principal = { Service = "lambda.amazonaws.com" }, Action = "sts:AssumeRole" }] })
  decrypt_statement = length(var.bootstrap_kms_key_arns) == 0 ? [] : [{ Effect = "Allow", Action = ["kms:Decrypt"], Resource = var.bootstrap_kms_key_arns, Condition = { StringEquals = { "kms:ViaService" = "secretsmanager.${var.region}.amazonaws.com" } } }]
}
resource "aws_iam_role" "host" {
  name_prefix        = "${var.name}-host-"
  assume_role_policy = local.ec2_trust
}
resource "aws_iam_instance_profile" "host" { role = aws_iam_role.host.name }
resource "aws_iam_role_policy" "host" {
  role = aws_iam_role.host.id
  policy = jsonencode({ Version = "2012-10-17", Statement = concat([
    { Effect = "Allow", Action = ["secretsmanager:GetSecretValue"], Resource = [var.host_bootstrap_secret_arn] },
    # SSM session channels require wildcard resources; there is no inbound SSH.
    { Effect = "Allow", Action = ["ssm:UpdateInstanceInformation", "ssmmessages:CreateControlChannel", "ssmmessages:CreateDataChannel", "ssmmessages:OpenControlChannel", "ssmmessages:OpenDataChannel"], Resource = ["*"] }
  ], local.decrypt_statement) })
}
resource "aws_iam_role" "controller" {
  name_prefix        = "${var.name}-controller-"
  assume_role_policy = local.lambda_trust
}
resource "aws_iam_role_policy" "controller" {
  role = aws_iam_role.controller.id
  policy = jsonencode({ Version = "2012-10-17", Statement = concat([
    { Effect = "Allow", Action = ["ec2:StartInstances", "ec2:StopInstances"], Resource = [aws_instance.host.arn] },
    { Effect = "Allow", Action = ["ec2:DescribeInstances"], Resource = ["*"] },
    { Effect = "Allow", Action = ["secretsmanager:GetSecretValue"], Resource = [var.controller_bootstrap_secret_arn] },
    { Effect = "Allow", Action = ["logs:CreateLogStream", "logs:PutLogEvents"], Resource = ["${aws_cloudwatch_log_group.controller.arn}:*"] }
  ], local.decrypt_statement) })
}
