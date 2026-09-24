resource "aws_cloudwatch_log_group" "controller" {
  name              = "/aws/lambda/${var.name}-capacity"
  retention_in_days = var.log_retention_days
}
resource "aws_lambda_function" "controller" {
  function_name                  = "${var.name}-capacity"
  role                           = aws_iam_role.controller.arn
  package_type                   = "Image"
  image_uri                      = var.controller_image_uri
  architectures                  = ["x86_64"]
  timeout                        = 45
  memory_size                    = 256
  reserved_concurrent_executions = var.controller_enabled ? 1 : 0
  environment {
    variables = {
      MOBILE_QA_POOL_ID              = var.pool_id
      MOBILE_QA_INSTANCE_ID          = aws_instance.host.id
      MOBILE_QA_API_ORIGIN           = var.api_origin
      MOBILE_QA_BOOTSTRAP_SECRET_ARN = var.controller_bootstrap_secret_arn
      DOPPLER_PROJECT                = var.doppler_project
      DOPPLER_CONFIG                 = var.controller_doppler_config
    }
  }
  depends_on = [aws_cloudwatch_log_group.controller, aws_iam_role_policy.controller]
}
resource "aws_apigatewayv2_api" "wake" {
  name          = "${var.name}-wake"
  protocol_type = "HTTP"
}
resource "aws_apigatewayv2_integration" "wake" {
  api_id                 = aws_apigatewayv2_api.wake.id
  integration_type       = "AWS_PROXY"
  integration_uri        = aws_lambda_function.controller.invoke_arn
  payload_format_version = "2.0"
  timeout_milliseconds   = 30000
}
resource "aws_apigatewayv2_route" "wake" {
  api_id    = aws_apigatewayv2_api.wake.id
  route_key = "POST /wake"
  target    = "integrations/${aws_apigatewayv2_integration.wake.id}"
}
resource "aws_apigatewayv2_stage" "wake" {
  api_id      = aws_apigatewayv2_api.wake.id
  name        = "$default"
  auto_deploy = true
  default_route_settings {
    throttling_burst_limit = 2
    throttling_rate_limit  = 1
  }
}
resource "aws_lambda_permission" "wake" {
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.controller.function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_api.wake.execution_arn}/*/POST/wake"
}
resource "aws_iam_role" "scheduler" {
  name_prefix        = "${var.name}-schedule-"
  assume_role_policy = jsonencode({ Version = "2012-10-17", Statement = [{ Effect = "Allow", Principal = { Service = "scheduler.amazonaws.com" }, Action = "sts:AssumeRole", Condition = { StringEquals = { "aws:SourceAccount" = data.aws_caller_identity.current.account_id } } }] })
}
resource "aws_iam_role_policy" "scheduler" {
  role   = aws_iam_role.scheduler.id
  policy = jsonencode({ Version = "2012-10-17", Statement = [{ Effect = "Allow", Action = ["lambda:InvokeFunction"], Resource = [aws_lambda_function.controller.arn] }] })
}
resource "aws_scheduler_schedule" "reconcile" {
  name                = "${var.name}-reconcile"
  state               = var.controller_enabled ? "ENABLED" : "DISABLED"
  schedule_expression = "rate(1 minute)"
  flexible_time_window { mode = "OFF" }
  target {
    arn      = aws_lambda_function.controller.arn
    role_arn = aws_iam_role.scheduler.arn
    input    = jsonencode({ source = "mobile-qa-scheduler", pool_id = var.pool_id })
    retry_policy {
      maximum_event_age_in_seconds = 60
      maximum_retry_attempts       = 1
    }
  }
}
resource "aws_cloudwatch_metric_alarm" "controller_errors" {
  for_each            = toset(["Errors", "Throttles"])
  alarm_name          = "${var.name}-capacity-${lower(each.value)}"
  namespace           = "AWS/Lambda"
  metric_name         = each.value
  dimensions          = { FunctionName = aws_lambda_function.controller.function_name }
  statistic           = "Sum"
  period              = 300
  evaluation_periods  = 1
  comparison_operator = "GreaterThanThreshold"
  threshold           = 0
  treat_missing_data  = "notBreaching"
  alarm_actions       = [var.alarm_topic_arn]
}
