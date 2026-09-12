# 06 — Generate reviewable, executable tests

Status: planned. Depends on: 05 and a qualified execution path. Owns: Rust source/requirement/generation services and jobs, typed model boundary, Tests generation/review UI.

## Deliverable

Given pasted user stories and acceptance criteria, propose a small case set, suite groups and a default plan. Every expected outcome has a source or an explicitly unapproved assumption. Generation saves drafts; it never approves or runs them automatically.

## Input

Persist a source revision containing the customer's text, app/environment context, declared critical journeys and constraints. Start with pasted text; repository crawling, ticket integrations and complex document ingestion are deferred.

Provide the generator with supported device capabilities, available fixture/reset adapters, permitted verification methods and existing cases. Do not send account secrets. Treat supplied text/app observations as data, not authority to expand tool permissions.

## Pipeline

1. **Normalize requirements:** extract criterion IDs, expected behavior, prerequisites and ambiguity. Preserve source anchors and contradictory statements for review.
2. **Propose scenarios:** generate happy paths and relevant negative cases grounded in those criteria. Bound count and scope to the pilot's critical journeys; avoid an unreviewable catalog.
3. **Validate structure:** parse typed output, apply business validation, reject unknown source IDs and unsupported checks. Repair malformed output only within a bounded retry budget.
4. **Check readiness:** map fixture/reset needs and execution limits. Separate “valid draft” from “executable on this configuration.” Missing knowledge becomes a question/needs-input state.
5. **Group and compare:** suggest suites/default plan, compare with existing cases and flag potential duplicates. Never merge or overwrite approved definitions automatically.
6. **Review:** show source → proposed expectation → verification method. Save edits and approval through spec 05. An approved selection becomes eligible for execution.

Optional device discovery is a later addition within this phase, only if grounding lacks screen context. Run it as a bounded device job with separate evidence and permissions. Observing the current app helps locate a control; it cannot establish whether a buggy behavior is correct.

## Job and failure behavior

Generation runs as a durable, bounded Rust background job and returns a job ID immediately. Persist source revision, prompt/template version, model/provider configuration, usage and per-stage output status. Limit request size, scenario count, model time, repair attempts and spend.

Canceling or retrying generation does not mutate already saved approved cases. Expose partial drafts with stage errors and allow a targeted retry. Key draft creation to generation/item IDs to avoid duplicate records on delivery retries. Do not hold a database transaction across model calls.

Use direct model API calls from Rust for structured drafting. Minitap's internal device reasoning remains in Python. OpenAI Agents SDK is unnecessary for the first pipeline; add orchestration only when measured workflows require it.

## UI

Tests → Generate → paste criteria → progress → review proposed cases. Show a summary of missing inputs, duplicates and unsupported checks before approval. Make “edit existing draft” and “generate proposed revision” explicit. Uploading a new APK reruns saved tests by default; it does not regenerate the library.

## Acceptance

Use fixed source fixtures: clear criterion, ambiguous behavior, contradictory source, missing reset, duplicate case and unsupported action. Check schema handling, valid source references, preserved uncertainty, partial errors and bounded retries. Stub provider responses in normal tests; run a small explicit real-model evaluation before pilot use.

Manually approved generated cases execute through the same manifest/worker/report path as authored cases. Seeded defects cannot be redefined as expected behavior by the generator. A schema-valid output alone does not satisfy this gate.
