# Mobile QA architecture source of truth

This directory is the canonical, version-controlled home for product requirements,
architecture, build specifications, operations and accepted engineering decisions.
Start here before planning or changing the application.

## Read by task

| Question                                                              | Authoritative document                                                             |
| --------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| What exists and what remains unverified?                              | [Current status](status.md)                                                        |
| Who is this for; what are the user stories, objects and result rules? | [Product requirements](product.md)                                                 |
| What owns each responsibility; what stack do we use?                  | [System architecture](system.md)                                                   |
| How do Rust, TypeScript and Python agree on data?                     | [Transport contracts](contracts.md)                                                |
| What comes first; what can run concurrently?                          | [Master implementation spec](implementation/00-master-spec.md) and its seven specs |
| How do we develop, validate and scope CI?                             | [Development workflow](development.md)                                             |
| How do we qualify the Android runner?                                 | [Device qualification](device-qualification.md)                                    |
| Where do we deploy first and how do we migrate later?                 | [Hosting](hosting.md)                                                              |
| How do secrets and environment selection work?                        | [Doppler environment](environment.md)                                              |
| Why did we choose this approach?                                      | [Decision record](decisions.md)                                                    |
| How should humans and AI document code?                               | [Commenting conventions](commenting.md)                                            |
| Which upstream packages and adaptations are in use?                   | [Dependency provenance](dependencies.md)                                           |

## Authority and maintenance

- Current explicit user instructions take precedence. Within project documentation,
  this directory supersedes old chat attachments, parent workspace specs, PR planning
  snapshots and the legacy docs links. Those are history or redirects, not competing specs.
- Documents define intended behavior. Source code, migrations, lockfiles and generated
  schemas establish what actually exists. If implementation and a document disagree,
  report the mismatch and reconcile it; do not claim a planned capability is implemented.
- Keep wire shapes in Rust, database changes in SeaORM migrations, dependency versions
  in locks, and CI selection in the workflow. Link to them instead of hand-copying their
  contents into another specification.
- Update the owning document in the same PR as a material product/architecture change.
  Add the reason to [decisions](decisions.md) when a lasting decision changes. Update
  [status](status.md) only with observed evidence; retain unresolved acceptance gates.
- Mark future behavior **planned**, product/commercial assumptions **proposed**, and
  working behavior **implemented**. An accepted design is not proof of runtime behavior.
- Keep the change small: add a focused section or spec when needed, not a second master
  document. New feature specs must name dependencies, owner boundaries and acceptance.
- Finish the whole scoped edit batch before checks. Documentation-only changes need
  link/consistency review, not application typechecks, builds or full test suites.

## Current direction

An operated Android regression-testing service: app setup → reviewed tests → run →
evidence report, with small Settings. Rust/Loco/SeaORM owns the application; React/Vite
owns the UI; Python/Minitap owns device interaction. Main's Mantine dashboard, Google
sign-in, workspace onboarding and app/APK setup are preserved in this branch. The local
Android runner and one live Minitap demo are verified independently. Tests/Runs,
UI-to-worker dispatch (04), full device qualification and hosted acceptance remain
open. See the status page before treating any milestone as complete.
