output "instance_id" { value = aws_instance.host.id }
output "wake_url" { value = "${aws_apigatewayv2_api.wake.api_endpoint}/wake" }
output "controller_function" { value = aws_lambda_function.controller.function_name }
output "root_volume_id" { value = aws_instance.host.root_block_device[0].volume_id }
