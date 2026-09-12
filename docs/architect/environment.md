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

`just dev` and `just dev-api` wrap the API with
`doppler run --no-fallback --forward-signals`. Injection happens in that child, so
secrets fetched for the API do not enter Vite's environment. `--no-fallback` disables
Doppler's local secret fallback file; fetching must succeed before the API starts.
`--forward-signals` preserves shutdown handling. No Doppler watch/restart loop is used.
The supervisor records startup failures in `.private/api.log` as before.

## Current variables

| Variable | Consumer and use |
|---|---|
| `HOST` | Production API public origin, required by production Loco config |
| `DATABASE_URL` | Production PostgreSQL connection, required by production Loco config |
| `MOBILE_USE_TELEMETRY_ENABLED` | Set to `false` before future real Minitap execution; explicit import smoke already sets this internally |

Development/test databases keep their isolated loopback configuration. They ignore
production DATABASE_URL, so selecting a production secret set cannot redirect a local
test at a customer database. Local Compose credentials are disposable development
configuration, not deployment credentials. Model/provider credentials will be specified
with the real worker implementation; no placeholder secret names or values are added.

The web app currently needs no secrets or public environment variables: it calls the
same-origin API. Any future `VITE_*` variable is public browser-build input; never store
server credentials under that prefix. Launch `just dev` normally, not under a parent
Doppler wrapper that would also pass server secrets to Vite.

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

## Spec 03 process settings

Production additionally requires `JWT_SECRET` (base64 of at least 64 random bytes),
`ARTIFACT_S3_ENDPOINT` (HTTPS), `ARTIFACT_S3_REGION`, `ARTIFACT_S3_BUCKET`,
`ARTIFACT_S3_ACCESS_KEY_ID` and `ARTIFACT_S3_SECRET_ACCESS_KEY`. Supply these through
Doppler only to the API. They have not been provisioned by this change. Production
uses private Railway Buckets with the logical backend `pilot`; an AWS S3 migration
copies immutable object keys/bytes and verifies stored SHA-256 before switching that
backend's endpoint/credentials. PostgreSQL stores metadata; APK bytes stay in storage.

Development/test always use private local disk and ephemeral JWT keys, even if the
parent has production credentials. Restart requires a new login and retains app/build
records. Dev artifacts live in ignored `.private/artifacts`; production scratch uses
`ARTIFACT_SCRATCH_DIR` or a process-host temporary directory. Scratch is disposable.
`MOBILE_QA_ANDROID_SDK` may select an explicitly installed read-only SDK; the default
is ignored `.private/android-sdk`. `JAVA_HOME` must select JDK 17. Validators receive
only this Java location plus a fixed PATH/locale, never the API secret environment.

Operator tasks consume `MOBILE_QA_OPERATOR_PASSWORD` or `MOBILE_QA_SECRET_LOCATOR`
only when needed. Passwords and locator values must not appear in task arguments.
Local synthetic smoke passes generated credentials directly in its subprocess
environment. `MOBILE_QA_TEST_SCOPE` is a test-only UUID that lets the owned HTTP
smoke process restart against the same private scratch/artifact directory.
