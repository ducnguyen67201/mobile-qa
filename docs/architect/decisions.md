# Accepted engineering decisions

These record the current baseline, not vendor guarantees or evidence of a finished
product. Update this record when the decision changes; implementation state belongs in
[status](status.md). All entries below were established on 2026-09-12.

| ID | Decision | Reason / consequence | Owner document |
|---|---|---|---|
| A01 | Operated Android QA pilot first | Deliver reviewed regression evidence before broad self-service/device support; buyer/pricing assumptions remain proposed | [Product](product.md) |
| A02 | Loco/Rust application with SeaORM/PostgreSQL | Reuse framework/ORM conventions; still maintain migrations and business validation | [System](system.md) |
| A03 | React/Vite, React Router, TanStack Query, Mantine | Packaged controls and a shared theme reduce local UI maintenance; no required SSR/Node production server | [System](system.md) |
| A04 | Runnable apps under apps; pure contracts under crates | Clear deployment and language boundaries, with one native toolchain per app | [System](system.md) |
| A05 | Rust/Utoipa → OpenAPI → Hey API SDK/Zod | Generate method/path/input/output types and validate runtime data; replaces ts-rs browser exports | [Contracts](contracts.md) |
| A06 | Rust/Schemars → JSON Schema → Pydantic | Validate worker transport without handwritten duplicate models | [Contracts](contracts.md) |
| A07 | Minitap owns device navigation; Python stays narrow | Avoid competing tap loops; Rust owns durable product state and result meaning | [Execution](implementation/04-execution-and-reports.md) |
| A08 | Author the complete scope before checking | Strict type/lint/test gates without repeated per-edit execution or check watchers | [Development](development.md) |
| A09 | CI selects affected apps and dependent contracts | Avoid unrelated suites; safe typechecking remains project/crate scoped; cancel superseded PR runs | [Development](development.md) |
| A10 | Doppler process injection; no env files | Keep runtime secrets outside source and browser processes; local checks require no secret access | [Environment](environment.md) |
| A11 | Prove one authored case before test generation | Establish install/reset/evidence/verdict behavior before automating test authoring | [Roadmap](implementation/00-master-spec.md) |
| A12 | docs/architect is the documentation authority | Keep a portable, versioned source of truth in the actual repository | [Index](README.md) |
| A13 | Railway private Buckets first for hosted APKs; AWS S3 later | User-selected storage direction; reusable S3-compatible adapter, stable keys/checksums and verified data migration; implementation/provisioning remain planned | [App setup](implementation/03-app-setup-and-ui-backend.md) |

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
