# Accepted engineering decisions

These record the current baseline, not vendor guarantees or evidence of a finished
product. Update this record when the decision changes; implementation state belongs in
[status](status.md). All entries below were established on 2026-09-12.

| ID  | Decision                                                    | Reason / consequence                                                                                                                                           | Owner document                                                    |
| --- | ----------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| A01 | Operated Android QA pilot first                             | Deliver reviewed regression evidence before broad self-service/device support; buyer/pricing assumptions remain proposed                                       | [Product](product.md)                                             |
| A02 | Loco/Rust application with SeaORM/PostgreSQL                | Reuse framework/ORM conventions; still maintain migrations and business validation                                                                             | [System](system.md)                                               |
| A03 | React/Vite, React Router, TanStack Query, Mantine           | Packaged controls and a shared theme reduce local UI maintenance; no required SSR/Node production server                                                       | [System](system.md)                                               |
| A04 | Runnable apps under apps; pure contracts under crates       | Clear deployment and language boundaries, with one native toolchain per app                                                                                    | [System](system.md)                                               |
| A05 | Rust/Utoipa → OpenAPI → Hey API SDK/Zod                     | Generate method/path/input/output types and validate runtime data; replaces ts-rs browser exports                                                              | [Contracts](contracts.md)                                         |
| A06 | Rust/Schemars → JSON Schema → Pydantic                      | Validate worker transport without handwritten duplicate models                                                                                                 | [Contracts](contracts.md)                                         |
| A07 | Minitap owns device navigation; Python stays narrow         | Avoid competing tap loops; Rust owns durable product state and result meaning                                                                                  | [Execution](implementation/04-execution-and-reports.md)           |
| A08 | Author the complete scope before checking                   | Strict type/lint/test gates without repeated per-edit execution or check watchers                                                                              | [Development](development.md)                                     |
| A09 | CI selects affected apps and dependent contracts            | Avoid unrelated suites; safe typechecking remains project/crate scoped; cancel superseded PR runs                                                              | [Development](development.md)                                     |
| A10 | Doppler process injection; no env files                     | Keep runtime secrets outside source and browser processes; local checks require no secret access                                                               | [Environment](environment.md)                                     |
| A11 | Prove one authored case before test generation              | Establish install/reset/evidence/verdict behavior before automating test authoring                                                                             | [Roadmap](implementation/00-master-spec.md)                       |
| A12 | docs/architect is the documentation authority               | Keep a portable, versioned source of truth in the actual repository                                                                                            | [Index](README.md)                                                |
| A13 | Railway private Buckets first for hosted APKs; AWS S3 later | User-selected storage direction; reusable S3-compatible adapter, stable keys/checksums and verified data migration; implementation/provisioning remain planned | [App setup](implementation/03-app-setup-and-ui-backend.md)        |
| A14 | Railway application hosting first; AWS later when needed    | Portable application image and PostgreSQL; Android/KVM hosting remains separately gated                                                                        | [Hosting](hosting.md)                                             |
| A15 | One shared immutable model registry and frozen assignment   | API resolves `key@revision` once; workers advertise exact compatibility, provider calls use the frozen snapshot, and no automatic fallback changes a run       | [Contracts](contracts.md)                                         |
| A16 | Versioned routing separate from physical device profiles    | Operator assignments change only new work; protocol 6 admits exact qualified sets and eligible jobs without rewriting saved reports                            | [Model capacity](implementation/09-model-catalog-and-capacity.md) |

No second general agent framework, queue platform or orchestration service is required
for the first implementation. Revisit these only with a concrete unmet requirement and
measured benefit. Device-cloud provider/SKU, model choice, price and qualified emulator profile
are still proposed or gated in their owning specs; this record does not approve spending.

## Dashboard component ownership (2026-09-12)

A03 now uses Mantine instead of copied shadcn/Tailwind primitives, following the
user's maintenance preference. Screens import packaged controls directly. Mantine
owns drawers, focus handling, menus, forms, responsive shell and table behavior;
`apps/web/src/theme.ts` owns product tokens/defaults and `index.css` owns the few
product surface styles. Do not introduce a second local primitive kit or class-string
variant system. The earlier shadcn plan/reviews remain historical snapshots.

## Google-only identity (2026-09-12)

Use Google Identity Services as the only sign-in provider. Retain Loco HMAC app
sessions, revocation, CSRF/origin checks and tenant grants. Verify Google assertions
with jsonwebtoken RS256 and Google's cached JWKS; never use tokeninfo for production
validation. Account identity is Google `sub`. Auto-link invited email only when Google
is authoritative (verified Gmail/Workspace); otherwise require explicit operator
linking. No public auto-registration is introduced. The follow-up migration removes
password hashes and revokes old sessions without removing product data.

## Open Google registration with approved workspace creation

Google verifies identity; account approval controls workspace creation independently.
Registration no longer needs an existing user or membership. Existing users remain
approved, new registrations start pending, and trusted operator/database approval
unlocks workspace creation. There is no browser self-approval permission. Users can
own multiple workspaces using the existing organizations/memberships model.

The selected workspace lives in the `workspace` URL parameter, not global mutable
session state. This supports independent tabs and browser history. Workspace list
queries include organization ID; nested resources retain authoritative server checks
against their persisted app and organization. Switching remounts transient UI state.

## Task-first mobile interaction (2026-09-13)

Following user feedback on the manual case editor, phase 06 starts with a phone
preview, a plain-language goal or selected control, and **Run task**. Minitap
plans and performs the task; saving a reusable test is optional afterward.
The accepted direction is still planned work. Existing phase 05 drafts remain
editable, and saved regression approval rules remain in force. Interactive task
completion does not establish an independently verified regression pass.
The [phase 06 specification](implementation/06-test-generation.md) owns the
session/job boundaries, generated contracts and acceptance criteria.

## Direct execution with explicit AI authoring — implemented in source (2026-09-13)

User feedback revises A07 and the task-first decision for the next phase 06 implementation.
Ordinary typed actions will use one Python direct-device executor shared by interactive
control, recording, discovery and approved regression runs. Minitap remains for explicit
Ask AI/legacy navigation. There is no automatic model fallback when a direct selector fails.

AI will discover bounded journeys and propose named, source-grounded test drafts; predefined
templates and direct reruns require no model. Rust owns durable jobs, authorization, typed
contracts, atomic saves and verdict semantics. Python owns bounded structured model calls in
an isolated child and validates proposed discovery actions before direct execution. This
supersedes the earlier proposed Rust-only model-calling generation pipeline and avoids
converting an opaque agent transcript into supposedly deterministic tests.

Preserve legacy payload hashes and approval rules. Generated expectations require explicit
review; app observations cannot establish intended behavior. See the
[owning spec](implementation/06-test-generation.md).
The source now follows this decision. Validation and live acceptance are tracked separately in status.md. Existing session payloads and receipt transactions own the additive state; a second queue and migration would duplicate those mechanisms. Invalid model output fails closed rather than buying automatic repair calls.

## 2026-09-20 — Save replaces human test approvals

Test authors requested one persistence action for cases, suites, release plans, templates
and selected AI proposals. Remove submit/fork/reviewer gates; complete Save creates or
reuses an immutable snapshot. Incomplete work remains durable and editable. Save never
means passed. Keep technical admission, exact version pins, operator permissions and
history. Migration 000008 preserves old decisions as legacy audit without fabricating
approvals for new saves. Retain internal draft storage/routes to limit wire churn;
namespace new mutation fingerprints so old retries fail closed rather than deserialize
obsolete receipts. Spec 05 owns the detailed upgrade and execution invariants.

## 2026-09-21 — Shared model registry and downstream resolution

Model identity is the immutable `(key, revision)` pair. The API-local registry stores only
nonsecret provider metadata and explicit capabilities. Historical profiles may carry a typed reference;
historical raw strings remain readable only through a bounded compatibility resolver. Creating
a model-enabled run or phone session freezes a complete `ResolvedModel` snapshot. Workers do not
select a model: protocol 5 advertises their qualified reference and compiled provider support,
and the API admits an exact match before a reservation or lease. Provider credentials remain in
the isolated Doppler child. There is no implicit model fallback or browser model picker.

## 2026-09-21 — Versioned model routing and qualified capacity

New execution profiles may omit model choice. An operator stages and activates an
immutable app/profile/purpose assignment revision; only new runs and phone sessions
freeze the new reference, while old jobs keep their exact model. Phone sessions can
freeze a separate structured-authoring model. Protocol 6 carries a bounded set of
host-qualified references and the API selects the oldest compatible queued job under
the existing reservation and fencing rules. A missing or incompatible worker leaves
the job queued with a computed explanation. Recovery reservations remain an explicit
operator gate. The catalog does not infer provider support or host qualification from
a model name.

## 2026-09-21 — Explicit smoke-suite run selection

A saved suite remains a collection of independently reset case versions;
its display order is not a shared-state workflow or claim priority. Each member
is a saved test case with expected checks; a raw action sequence cannot be a
suite member. The first repeatable smoke
delivery asks the tester to select a build and qualified device explicitly and
pins an optional completed baseline run at submission. This replaces the current
suite button's implicit newest-build/first-profile choice. The existing run
manifest, attempt, evidence and comparison pipeline owns the result; no second
suite engine or approval step is added. The API remains the worker task picker:
it filters by worker compatibility, claims the oldest eligible run and then any
eligible saved-suite case under app and device reservations, and waits for
verified cleanup before another claim. Stable attempt IDs select among suite
cases without using the editor's order; release plans keep their case order.
Stable run IDs make equal timestamps
deterministic; an incompatible older run does not block eligible work. The
[focused spec](implementation/10-reliable-smoke-suite.md) defines the business
rules and acceptance boundary.
