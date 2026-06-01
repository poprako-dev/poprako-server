# poprako-r Refactor Workflow

This document defines the migration workflow from `references/poprako-s` to
`poprako-r`. The target style is domain-by-domain, but not every domain can be
completed vertically in isolation. Some use cases are inherently cross-domain,
so the work must be split into stable lower-layer contracts first, then closed
with use cases and API integration once all dependencies exist.

## Ground Rules

- Always read the corresponding Go code before implementing a Rust feature.
- Use project-local skills from `.pi/skills/`, not Codex global skill folders.
- Keep source comments in English only.
- Do not mention Go reference paths or Go implementation details in source
  comments.
- Prefer minimal domain query traits. Add methods only when a caller needs them.
- Publish domain events only after the surrounding transaction commits.
- Keep harness implementations as pure delegation.

Relevant local skill files:

- `.pi/skills/aggregate-definition-spec/SKILL.md`
- `.pi/skills/poprako-aggr-conventions/SKILL.md`
- `.pi/skills/query-domain-spec/SKILL.md`
- `.pi/skills/query-infra-spec/SKILL.md`
- `.pi/skills/poprako-conventions/SKILL.md`
- `.pi/skills/error-handling-spec/SKILL.md`
- `.pi/skills/trait-def-spec/SKILL.md`
- `.pi/skills/tracing-usage-spec/SKILL.md`
- `.pi/skills/thirdparty-macro-usage-spec/SKILL.md`
- `.pi/skills/harness-spec/SKILL.md`

## Current Baseline

Rust already has partial coverage for these foundation domains:

- `user`
- `team`
- `member`
- `member_invitation`
- `system_mail`

The remaining Go domains are mostly content/workflow domains:

- `workset`
- `comic`
- `chapter`
- `page`
- `unit`
- `assignment`
- `assignment_invitation`
- `announcement`
- `comment`
- `chapter_port`
- `oss_msg` or its Rust replacement in the effect layer

The content domains are strongly coupled:

- Workset creation depends on team membership and team-scoped index allocation.
- Comic creation creates a comic, first chapter, reviewer assignment, and counter
  updates in one transaction.
- Chapter creation creates a chapter, reviewer assignment, and comic counter
  updates in one transaction.
- Chapter workflow emits events that update notifications and pinned comic
  workflow state.
- Page and unit operations update page/chapter counters.
- Workset/chapter deletion requires cascade cleanup and image deletion effects.

Because of those dependencies, a strict "finish one domain top to bottom, then
start the next" sequence will either block or force temporary hacks. Use the
two-track workflow below instead.

## Two-Track Workflow

Every domain goes through the same local pipeline, but use-case/API closure is
delayed until all cross-domain dependencies are available.

### Track A: Domain Contract Slice

This slice creates stable lower-layer contracts. It should be small and compile
cleanly.

1. Read Go aggregate, repo interface, service, app implementation, infra repo,
   events, and migrations for the target domain.
2. Write a short implementation note in the issue or PR description:
   aggregate fields, nullable fields, counters, unique constraints, events,
   permission checks, and cross-domain writes.
3. Add or update migration SQL.
4. Regenerate `src/infrastructure/query/schema.rs`.
5. Add domain values/enums if needed.
6. Add aggregate structs under `src/domain/model/aggregate/`.
7. Add domain events under `src/domain/model/event/` if the aggregate emits
   events.
8. Add query traits under `src/domain/query/`.
9. Register transactional traits in `src/domain/query.rs`.
10. Add compound functions under `src/domain/compound/` only for operations that
    can fail without doing I/O or that coordinate pure cross-aggregate domain
    logic.
11. Add domain tests for aggregate and compound behavior.
12. Run `cargo check` and targeted tests.

Track A does not require HTTP handlers and does not require completing every use
case for that domain.

### Track B: Vertical Use-Case Slice

This slice closes one user-visible behavior end to end.

1. Confirm all required Track A contracts exist for every participating domain.
2. Add or update infra query entities and query functions.
3. Add infra query trait impls.
4. Add effect handlers if the use case emits events.
5. Add usecase value objects.
6. Add usecase function with transaction boundaries.
7. Add fake-harness usecase unit tests.
8. Wire harness delegation for any newly required trait.
9. Add API handler/router changes.
10. Add API integration tests.
11. Run `cargo check`, targeted tests, and then broader test suite.

Track B should not introduce new schema concepts unless unavoidable. If it does,
split that schema change back into Track A first.

## Recommended Implementation Order

### Phase 0: Foundation Hardening

Goal: make lower-layer work repeatable and testable.

Deliverables:

- Confirm migration/schema workflow is working locally.
- Add shared pagination/filter value objects if missing.
- Stabilize role mask and workflow enum/value definitions.
- Stabilize `Event`, `EventSink`, `EventEmit`, `EffectSink`, and post-commit
  effect dispatch patterns.
- Add a lightweight fake harness pattern for usecase unit tests.
- Add a DB integration-test harness before large infra query work.

Validation:

- `cargo check`
- domain/value unit tests
- at least one fake-harness usecase test
- at least one infra query integration test or documented test blocker

### Phase 1: Identity And Membership Closure

Goal: finish the foundation domains because every later permission check depends
on them.

Order:

1. `user`
2. `system_mail`
3. `team`
4. `member_invitation`
5. `member`

Required use cases before moving on:

- User sign-up/login/current-user lookup.
- Team create/get/list/update/avatar reservation/upload confirmation.
- Member invitation create/list/update/delete.
- Member create/list/update-role/delete/join-team.
- System mail list and mark-read.

Important seams:

- User sign-up must atomically consume one pending member invitation and create
  the initial member row.
- User registration event must create a system mail for the inviter after commit.
- Team/member permission checks become reusable compound/domain functions for
  later domains.

Validation:

- domain tests for role masks and invitation state.
- usecase tests for sign-up race-sensitive behavior with fake transactional
  query.
- infra tests for unique pending invitation constraints.
- API tests for auth-required and permission-denied paths.

### Phase 2: Content Spine Contracts

Goal: add the persistent model and query contracts for the central hierarchy
before closing cross-domain use cases.

Order:

1. `workset`
2. `comic`
3. `chapter`
4. `assignment`
5. `page`
6. `unit`

Why this order:

- Workset depends only on team/member foundation.
- Comic depends on workset but full comic creation also needs chapter and
  assignment.
- Chapter depends on comic and has the critical workflow/pinned invariants.
- Assignment depends on chapter and is required by chapter/page/unit permissions.
- Page depends on chapter and is required by cover fallback and import/export.
- Unit depends on page and chapter counters.

Track A deliverables for this phase:

- Workset aggregate/query/infra with team-scoped `workset_next_index`.
- Comic aggregate/query/infra with title search fields, counters, cover fields,
  and chapter index allocation.
- Chapter aggregate/query/infra with workflow timestamps, pinned state, page/unit
  counters, and unique pinned-chapter behavior.
- Assignment aggregate/query/infra with timed role fields and create/update/delete
  semantics.
- Page aggregate/query/infra with image reservation and upload confirmation.
- Unit aggregate/query/infra with save/reindex/delete semantics and translated /
  proofread count derivation.

Critical invariants:

- Creating a chapter must atomically clear any existing pinned chapter for the
  same comic and insert the new chapter as pinned.
- Updating a chapter to pinned must atomically unpin other chapters in the same
  comic.
- Comic workflow filters must be based on the current pinned chapter state. If
  Rust adopts replica columns, only event/effect handlers may write those
  replica fields.
- Counter changes must be atomic and bounded where the Go behavior requires it.
- Nullable workflow timestamps must stay nullable through domain, infra, and API
  mappings.

Validation:

- aggregate tests for chapter workflow transitions.
- infra tests for pinned uniqueness under repeated chapter creation.
- infra tests for counter deltas and page/unit count updates.
- query tests for list filters and pagination.

### Phase 3: Content Use-Case Closure

Goal: close user-visible content behavior once Phase 2 contracts exist.

Recommended vertical slices:

1. Workset list/create/update.
2. Comic list/get/update.
3. Chapter list/get/get-pinned.
4. Assignment list/upsert/delete.
5. Chapter create.
6. Comic create.
7. Page reserve/list/mark-uploaded.
8. Unit list/save.
9. Chapter workflow update.
10. Chapter delete.
11. Workset delete.

Why this order:

- Read/list/update endpoints validate query contracts before complex
  transactions are added.
- Assignment must exist before chapter/page/unit permission checks are useful.
- Chapter create is simpler than comic create and proves chapter + assignment +
  comic counter coordination.
- Comic create reuses the proven chapter-create and assignment behavior.
- Delete flows come late because they require cascade logic and image cleanup.

Transaction boundaries:

- Workset create: permission check, team index allocation, workset insert.
- Chapter create: permission check, comic chapter index allocation, chapter
  insert, comic chapter count update, comic last-active touch, reviewer
  assignment insert, post-commit assignment event.
- Comic create: permission check, workset comic index allocation, comic insert,
  workset comic count update, first chapter insert, comic chapter count update,
  comic last-active touch, reviewer assignment insert, post-commit assignment
  event.
- Page reservation: permission check, page batch insert or image key reservation,
  chapter page count update, image creation effect enqueue if used.
- Unit save: permission check, diff existing units, apply create/update/delete,
  reindex if needed, update page and chapter unit counters.
- Delete flows: load descendants, enqueue image deletion effects, delete children
  before parents, emit post-commit removal events.

Validation:

- fake-harness usecase tests for every transaction boundary.
- infra integration tests for each multi-row transaction.
- API tests for happy path, unauthorized, forbidden, not found, and conflict.

### Phase 4: Workflow Events And Effects

Goal: move asynchronous behavior behind Rust effect handlers and keep it
post-commit.

Events to support:

- User signed up.
- Assignment created.
- Assignment removed.
- Chapter workflow completed.
- Chapter published.
- Chapter removed.

Effect handlers:

- Notify inviter when an invitation is consumed.
- Notify next-phase assignees after workflow completion.
- Update pinned comic workflow projection if Rust schema uses projection fields.
- Update user stats if that feature is enabled.
- Clean created/deleted image objects if Rust keeps an OSS message queue.

Rules:

- Aggregates collect events privately.
- Use cases publish effects only after successful commit.
- Effect handlers must be idempotent or tolerate duplicate delivery.
- Effect handler failures must be logged but must not roll back the already
  committed user-facing transaction.

Validation:

- unit tests for event emission.
- fake effect-sink tests that verify events are not published on rollback.
- handler tests for malformed payloads and missing target rows.

### Phase 5: Collaboration And Import/Export

Goal: implement features that depend on the complete content spine.

Order:

1. `assignment_invitation`
2. `chapter_port`
3. `announcement`
4. `comment`

Rationale:

- Assignment invitation depends on user, chapter, assignment, and permission
  checks.
- Chapter import/export depends on chapter, page, and unit.
- Announcement and comment depend mostly on team/member permissions and can be
  done earlier if needed, but they do not unblock the main content workflow.

Validation:

- parser/formatter tests for chapter import/export.
- usecase tests for assignment invitation invite/join/delete.
- API integration tests for import/export payload compatibility.

## Per-Domain Checklist

Use this checklist for every domain PR.

- Go reference files read: aggregate, repo interface, service, app, infra repo,
  entity, events, migrations, HTTP handler.
- Domain invariants recorded before coding.
- Migration updated and schema regenerated.
- Aggregate has exactly one `*Aggr` read model and uses `PrivateMarker`.
- Input structs use `Form`, `Update`, or `Patch` suffixes.
- Domain query traits are split into read-only and transactional traits.
- Transactional trait registered in `src/domain/query.rs` when needed.
- Infra entity structs use `Entry`, `Row`, `Aspect`, or `Snapshot` suffixes.
- Diesel inserts/selects use entity structs, not inline tuples.
- Expected errors use i18n keys and `trace_debug`.
- Unrecoverable errors include location prefix and `trace_error`.
- Constructors and pure domain logic do not use `#[instrument]`.
- Harness changes are pure delegation.
- Domain tests exist for pure rules.
- Usecase fake-harness tests exist for orchestration.
- Infra tests exist for DB constraints and transaction behavior.
- API tests exist before marking the vertical slice complete.

## Handoff Contract Between Layers

Each layer should hand a stable contract to the next layer.

Domain aggregate to domain query:

- Concrete aggregate fields, nullable semantics, constructors, events, and pure
  state transitions are fixed.

Domain query to infra query:

- Trait methods describe caller needs only.
- Method names encode business intent, not CRUD completeness.
- Transactional methods are only those that require row locks, atomic writes, or
  transaction-scoped coordination.

Infra query to usecase:

- Query functions return domain aggregates, not Diesel rows.
- Not-found and conflict behavior is mapped to `DomainError::Expected`.
- DB failures are mapped to `DomainError::Unrecoverable`.
- Multi-row invariants are enforced in DB transactions, not only in memory.

Usecase to API:

- Usecase value objects are stable and validated.
- Permission errors and argument errors are distinguishable.
- Domain events are emitted only after commit.
- API handlers contain request/response mapping only, not business logic.

API to integration tests:

- Tests cover request shape, auth extraction, status mapping, and response shape.
- Business behavior is already covered below API; API tests should not duplicate
  every domain edge case.

## Recommended PR Sizing

Prefer these PR shapes:

- One domain Track A PR: migration, aggregate, query traits, infra query, domain
  and infra tests.
- One vertical Track B PR: usecase, effect, harness, API, usecase/API tests.
- One shared-foundation PR: values, event/effect infrastructure, test harness.

Avoid these PR shapes:

- A PR that adds schema, aggregate, infra, usecase, API, and cross-domain effects
  for several domains at once.
- A PR that adds generic CRUD methods "for later".
- A PR that implements a high-level usecase before all participating lower-layer
  contracts exist.

## Practical Next Steps

1. Finish Phase 0 test harness and schema regeneration workflow.
2. Audit Phase 1 existing code against the per-domain checklist.
3. Start Phase 2 with `workset` Track A.
4. Continue Phase 2 through `comic`, `chapter`, `assignment`, `page`, and `unit`
   contracts before closing complex create/delete use cases.
5. Close Phase 3 vertical slices in the listed order.
