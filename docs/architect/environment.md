# Process environment through Doppler

Doppler is the source of runtime secrets. Do not create `.env`, `.env.local`, or
`.env.example`; they are all ignored. Vite explicitly disables automatic env-file
loading. The Python SDK still depends on python-dotenv internally; that dependency
is not a project environment-file workflow and its security override stays pinned.

## Developer setup

Install the Doppler CLI, then run from the repository root:

```sh
doppler login
doppler setup
just dev
```

The project is `mobile-qa`; the current local selection is `dev`. Root configs `dev`,
`stg` and `prd` exist; Doppler also created `dev_personal`. Directory-scoped selection
lives in the CLI's user configuration. To select it explicitly:

```sh
doppler setup --project mobile-qa --config dev --no-interactive --no-save-token
```

Live process injection from `apps/api` was verified on 2026-09-12 without printing secret
values. Project creation does not mean production database/model credentials are populated. Existing CLI
credentials are reused. Never paste tokens into source, command arguments or logs.
`just setup` installs application dependencies and does not modify Doppler configuration.

`just dev` wraps API and workers separately; `just dev-api` wraps only the API with
`doppler run --no-fallback --forward-signals`. Injection happens in that child, so
secrets fetched for the API do not enter Vite's environment. `--no-fallback` disables
Doppler's local secret fallback file; fetching must succeed before the API starts.
`--forward-signals` preserves shutdown handling. No Doppler watch/restart loop is used.
The full-stack supervisor records startup failures in `.private/dev/latest/`; `just dev-ui`
retains `.private/api.log` and `.private/web.log`. The nonsecret dev config chooses the API
Doppler scope, while the qualified host profile chooses worker/model scope.

## Current variables

| Variable                       | Consumer and use                                                                                            |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------- |
| `HOST`                         | Production API public origin, required by production Loco config                                            |
| `DATABASE_URL`                 | Production PostgreSQL connection, required by production Loco config                                        |
| `MOBILE_USE_TELEMETRY_ENABLED` | Set to `false` before future real Minitap execution; explicit import smoke already sets this internally     |
| `STRIPE_SECRET_KEY`            | API-only Stripe Checkout session creation and subscription verification; optional until checkout is enabled |
| `STRIPE_WEBHOOK_SECRET`        | API-only signed paid-invoice webhook verification; must be present together with the secret key             |

Development/test databases keep their isolated loopback configuration. They ignore
production DATABASE_URL, so selecting a production secret set cannot redirect a local
test at a customer database. Local Compose credentials are disposable development
configuration, not deployment credentials. Model/provider credentials will be specified
with the real worker implementation; no placeholder secret names or values are added.

The web app needs no secrets or public environment variables: it calls the
same-origin API. Any future `VITE_*` variable is public browser-build input; never store
server credentials under that prefix. Launch `just dev` normally, not under a parent
Doppler wrapper that would also pass server secrets to Vite.

To enable monthly checkout in an environment, inject both Stripe variables into
the API process through Doppler and register the environment's
`/api/billing/stripe-webhook` endpoint for `invoice.paid`. The app returns a
service-unavailable error when either variable is absent; selecting a card does
not grant credits. Use provider test mode before live activation.

## CI and deployment

Typechecks, lint, tests, generation, builds, fake-worker runs and local smoke checks need
no Doppler token or project access. Existing PR jobs retain that behavior. They use no
secret files or real cloud/device/model credentials.

A future deployment should inject a config-scoped service credential through its
platform secret store, then launch the API process through Doppler:

```sh
cd apps/api
doppler run --no-fallback --forward-signals -- ../../target/release/mobile-qa-cli start --environment production
```

This is a launch pattern, not an implemented deployment pipeline. A real worker launcher
must use its own least-privilege Doppler config when that worker is implemented. Do not
introduce secret downloads, mounted env files, or tokens in build artifacts.

References: [Doppler CLI](https://docs.doppler.com/docs/cli),
[Doppler configuration](https://docs.doppler.com/docs/environment-based-configuration),
[Vite envDir](https://vite.dev/config/shared-options#envdir).

## Phase 02 SDK child

The explicit device runner injects `OPENAI_API_KEY` through Doppler into its SDK child.
The qualified registry reference lives in the nonsecret device profile as a `[model_ref]`
table containing `key` and `revision`. Provider model identity comes from the API-frozen
`ResolvedModel`, not host TOML. The child drops unrelated
server credentials and tracing exports before SDK import, forces telemetry=false and
PYTHON_DOTENV_DISABLED=1, and uses a private cwd with no env file. Emulator, fixture
backend, doctor and offline checks never fetch model secrets. See
[device qualification](device-qualification.md). The verified local demo uses the
operator-selected `mobile-qa / dev_personal` Doppler config in an ignored device
profile; no key is committed. API and worker configs remain independently scoped.

## Spec 03 process settings

Production additionally requires `JWT_SECRET` (base64 of at least 64 random bytes),
`ARTIFACT_S3_ENDPOINT` (HTTPS), `ARTIFACT_S3_REGION`, `ARTIFACT_S3_BUCKET`,
`ARTIFACT_S3_ACCESS_KEY_ID` and `ARTIFACT_S3_SECRET_ACCESS_KEY`. Supply these through
Doppler only to the API. They have not been provisioned by this change. Production
uses private Railway Buckets with the logical backend `pilot`; an AWS S3 migration
copies immutable object keys/bytes and verifies stored SHA-256 before switching that
backend's endpoint/credentials. PostgreSQL stores metadata; APK bytes stay in storage.

Multipart uploads are enabled automatically for remote object storage; no separate
environment flag is required. Verify hosted signing and browser CORS before
customer rollout. Local storage retains the streaming upload path.

Development/test always use private local disk and ephemeral JWT keys, even if the
parent has production credentials. Restart requires a new login and retains app/build
records. Dev artifacts live in ignored `.private/artifacts`; production scratch uses
`ARTIFACT_SCRATCH_DIR` or a process-host temporary directory. Scratch is disposable.
`MOBILE_QA_ANDROID_SDK` may select an explicitly installed read-only SDK; the default
is ignored `.private/android-sdk`. `JAVA_HOME` must select JDK 17. Validators receive
only this Java location plus a fixed PATH/locale, never the API secret environment.

Operator reference tasks consume `MOBILE_QA_SECRET_LOCATOR` only when needed.
Locator values must not appear in task arguments. Google-only sign-in has no local
password, password reset task, or operator password environment variable.
Local synthetic smoke keeps signing keys ephemeral and injects only fixture public
verification keys into its test API subprocess environment. `MOBILE_QA_TEST_SCOPE` is a test-only UUID that lets the owned HTTP
smoke process restart against the same private scratch/artifact directory.

Google sign-in requires `GOOGLE_CLIENT_ID`, injected into the API via Doppler. The API
exposes this public identifier with the browser-bound login challenge; it is not a
secret. No Google client secret is used. Development defaults to
`http://localhost:5173`; `MOBILE_QA_DEV_ORIGIN` optionally selects another loopback
origin for Google's localhost registration. Tests ignore this override.
See [Google setup](development.md#google-only-sign-in). Without a configured client,
Google sign-in returns an explicit unavailable error and offers no password fallback.

## Execution worker injection

The API stores only worker token digests. `MOBILE_QA_WORKER_TOKEN` is injected into
worker registration and the polling worker process; it is never a CLI argument or
journal value. The client accepts HTTPS, with HTTP allowed only for loopback local
checks, and refuses redirects. Lease tokens stay in memory. Local child job files
contain the nonsecret frozen manifest without API credentials.

Real navigation uses the qualified host profile's explicit Doppler project/config
for the SDK child, matching phase 02. The child environment excludes API/lease/DB
credentials. The fake HTTP smoke supplies isolated synthetic tokens and never calls
Doppler, starts an emulator or calls a model. No .env files are introduced.

## Device capacity runtime scopes

The API operator registration task consumes `MOBILE_QA_HOST_TOKEN` and
`MOBILE_QA_CAPACITY_CONTROL_TOKEN` from its injected process. Only hashes are stored.
The host receives only its host credential, obtains app/profile-scoped worker grants,
and never receives database or object-store credentials. The controller receives
its separate control credential and `MOBILE_QA_CAPACITY_WAKE_SECRET`; the API's
outbox dispatcher receives that same wake secret and `MOBILE_QA_CAPACITY_WAKE_URL`.
The wake secret signs a request; it cannot substitute for host or controller access.

AWS Secrets Manager stores scoped Doppler bootstrap tokens selected by ARN. The
bootstrap process fetches a token into memory and execs Doppler with no fallback
file. OpenTofu passes only ARN and nonsecret scope identifiers. Host/API/Lambda
images contain no runtime secrets. Local validation removes AWS and Doppler
credentials and disables EC2 metadata lookup.

`PORT` is a nonsecret platform setting used by production routing (the application
image defaults to 5150). `MOBILE_QA_APK_CACHE_ROOT` selects a private retained disk
cache; it is not a credential or a durable artifact store. Slot socket/token values
are ephemeral process capabilities and must not appear in profiles, logs or images.

## Direct-only device profiles

Install worker extras with `uv sync --project apps/mobile-worker --frozen --extra device
--extra ai --extra sdk`. A direct-only host profile omits `model_ref`; register its API
execution profile with driver `direct` and no `model` field. Direct commands need no model secret.
AI authoring requires an explicitly configured model and a Minitap-capable profile; only its
isolated child is wrapped in Doppler. Ordinary tests and synthetic smoke remain Doppler-free.
Use `just dev` for the complete local stack or the owned-device task-worker command
documented in spec 06; model-capable workers advertise the exact reference with protocol 5.
