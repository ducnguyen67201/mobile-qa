import pytest

from capacity_controller.bootstrap import command


def test_only_fixed_runtime_commands_are_allowed():
    assert command("host")[-2:] == ["--config", "/etc/mobile-qa/host.toml"]
    assert command("lambda")[-1] == "capacity_controller.handler.handler"
    with pytest.raises(ValueError):
        command("sh")


def test_lambda_bootstrap_passes_token_only_in_child_environment(monkeypatch):
    from capacity_controller import bootstrap

    token = "dp.st.test.synthetic-bootstrap-token"
    secret_arn = "arn:aws:secretsmanager:us-east-1:123456789012:secret:bootstrap"
    monkeypatch.setattr(bootstrap.sys, "argv", ["bootstrap", "lambda"])
    for key, value in {
        "MOBILE_QA_BOOTSTRAP_SECRET_ARN": secret_arn,
        "AWS_REGION": "us-east-1",
        "DOPPLER_PROJECT": "mobile-qa",
        "DOPPLER_CONFIG": "test",
    }.items():
        monkeypatch.setenv(key, value)
    monkeypatch.delenv("DOPPLER_TOKEN", raising=False)

    class Secrets:
        def get_secret_value(self, *, SecretId):
            assert SecretId.endswith(":secret:bootstrap")
            return {"SecretString": token}

    monkeypatch.setattr(bootstrap.boto3, "client", lambda *args, **kwargs: Secrets())
    captured = []
    monkeypatch.setattr(bootstrap.os, "execve", lambda *args: captured.append(args))
    bootstrap.main()
    executable, arguments, environment = captured[0]
    assert executable == "/usr/local/bin/doppler"
    assert "--no-fallback" in arguments and "--forward-signals" in arguments
    assert all(token not in argument for argument in arguments)
    assert environment["DOPPLER_TOKEN"] == token
    assert "DOPPLER_TOKEN" not in bootstrap.os.environ
    assert arguments[-1] == "capacity_controller.handler.handler"


@pytest.mark.parametrize("token", ["not-a-service-token", "dp.st.unsafe\nvalue"])
def test_invalid_bootstrap_token_never_launches_runtime(monkeypatch, token):
    from capacity_controller import bootstrap

    monkeypatch.setattr(bootstrap.sys, "argv", ["bootstrap", "lambda"])
    for key in (
        "MOBILE_QA_BOOTSTRAP_SECRET_ARN",
        "AWS_REGION",
        "DOPPLER_PROJECT",
        "DOPPLER_CONFIG",
    ):
        monkeypatch.setenv(key, "synthetic")

    class Secrets:
        def get_secret_value(self, **kwargs):
            return {"SecretString": token}

    monkeypatch.setattr(bootstrap.boto3, "client", lambda *args, **kwargs: Secrets())
    monkeypatch.setattr(bootstrap.os, "execve", lambda *args: pytest.fail("invalid token was used"))
    with pytest.raises(ValueError, match="invalid_bootstrap_token"):
        bootstrap.main()
