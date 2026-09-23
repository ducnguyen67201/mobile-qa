# syntax=docker/dockerfile:1
# Context is repository root. Verified Linux/amd64 Lambda Python 3.12 manifest.
ARG LAMBDA_IMAGE=public.ecr.aws/lambda/python:3.12@sha256:3fca070d5384ec087f43ebc81465d93a0b03c1f6fc4c060ffc2200158d19b487
FROM ${LAMBDA_IMAGE}
RUN dnf install --assumeyes tar gzip curl-minimal && dnf clean all
COPY infra/deployment/install_runtime.sh /tmp/install_runtime.sh
RUN sh /tmp/install_runtime.sh && rm /tmp/install_runtime.sh
COPY infra/aws/capacity-controller/pyproject.toml infra/aws/capacity-controller/uv.lock /opt/controller/
RUN uv sync --project /opt/controller --frozen --no-dev --no-install-project --python /var/lang/bin/python
ENV PATH="/opt/controller/.venv/bin:${PATH}" \
    PYTHONPATH="/opt/controller/.venv/lib/python3.12/site-packages:/var/task"
COPY infra/aws/capacity-controller/capacity_controller ${LAMBDA_TASK_ROOT}/capacity_controller
COPY apps/mobile-worker/src/mobile_qa_worker/generated ${LAMBDA_TASK_ROOT}/mobile_qa_worker/generated
ENTRYPOINT ["/var/lang/bin/python", "-m", "capacity_controller.bootstrap", "lambda"]
