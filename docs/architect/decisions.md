# Accepted engineering decisions

These record the current baseline, not vendor guarantees or evidence of a finished
product. Update this record when the decision changes; implementation state belongs in
[status](status.md). All entries below were established on 2026-09-12.

| ID | Decision | Reason / consequence | Owner document |
|---|---|---|---|
| A01 | Operated Android QA pilot first | Deliver reviewed regression evidence before broad self-service/device support; buyer/pricing assumptions remain proposed | [Product](product.md) |
| A02 | Loco/Rust application with SeaORM/PostgreSQL | Reuse framework/ORM conventions; still maintain migrations and business validation | [System](system.md) |
| A03 | React/Vite, React Router, TanStack Query, Tailwind/shadcn | Keep the working starter and quick UI iteration; no required SSR/Node production server | [System](system.md) |
| A04 | Runnable apps under apps; pure contracts under crates | Clear deployment and language boundaries, with one native toolchain per app | [System](system.md) |
| A05 | Rust/Utoipa → OpenAPI → Hey API SDK/Zod | Generate method/path/input/output types and validate runtime data; replaces ts-rs browser exports | [Contracts](contracts.md) |
| A06 | Rust/Schemars → JSON Schema → Pydantic | Validate worker transport without handwritten duplicate models | [Contracts](contracts.md) |
| A07 | Minitap owns device navigation; Python stays narrow | Avoid competing tap loops; Rust owns durable product state and result meaning | [Execution](implementation/04-execution-and-reports.md) |
| A08 | Author the complete scope before checking | Strict type/lint/test gates without repeated per-edit execution or check watchers | [Development](development.md) |
| A09 | CI selects affected apps and dependent contracts | Avoid unrelated suites; safe typechecking remains project/crate scoped; cancel superseded PR runs | [Development](development.md) |
| A10 | Doppler process injection; no env files | Keep runtime secrets outside source and browser processes; local checks require no secret access | [Environment](environment.md) |
| A11 | Prove one authored case before test generation | Establish install/reset/evidence/verdict behavior before automating test authoring | [Roadmap](implementation/00-master-spec.md) |
| A12 | docs/architect is the documentation authority | Keep a portable, versioned source of truth in the actual repository | [Index](README.md) |

No second general agent framework, queue platform or orchestration service is required
for the first implementation. Revisit these only with a concrete unmet requirement and
measured benefit. Cloud provider/SKU, model choice, price and qualified emulator profile
are still proposed or gated in their owning specs; this record does not approve spending.
