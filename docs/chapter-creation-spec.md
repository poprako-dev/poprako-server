# Spec: Shared chapter creation obligations

Status: Implementation authorized on 2026-09-23; not published to an issue tracker.

Branch: `fix/architecture-review` carries all three architecture-review fixes.
This is the second item, following shared Unit sequence reconstruction.

## Problem Statement

Maintainers must currently keep two implementations of chapter creation in sync:
creating a Comic also creates its first Chapter, while standalone Chapter
creation adds a Chapter to an existing Comic. Both paths encode index allocation,
default subtitle selection, creation and pinning, chapter counters, Comic
activity, optional worker assignments, and creation history.

These obligations form one business outcome but have different ownership in
the two implementations. In particular, the Comic path's creator-assignment
helper also records Chapter creation even when no assignment is requested.
Its caller must know an obligation that the helper's name does not express.

Future changes can update one path while leaving the other incomplete. No
specific production defect has been established; the problem is duplicated
business knowledge and the effort required to preserve consistent behavior.

## Solution

Concentrate the shared Chapter creation obligations in one internal use-case
module used by both creation paths. The caller retains authorization and owns
the transaction; the shared implementation completes the Chapter creation
within that existing transaction.

Preserve the meaningful difference between creating a first Chapter under a
new Comic and replacing the pinned Chapter under an existing Comic. Preserve
permissions, lock protection, history, counters, error behavior, and rollback.

## User Stories

1. As a maintainer, I want one owner for shared Chapter creation obligations,
   so that a business-rule change reaches both creation paths.
2. As a maintainer, I want a creation module whose responsibility includes its
   required history, so that callers do not depend on hidden side effects of an
   assignment helper.
3. As an authorized team administrator, I want Comic creation to include its
   first Chapter, so that a successful result is immediately usable.
4. As an authorized team administrator, I want to add a Chapter to an existing
   writable Comic, so that its work can continue under the same rules.
5. As a team administrator, I want Chapters assigned the established next index,
   so that their sequence remains correct.
6. As a team administrator, I want an omitted subtitle to use the existing
   default rule, so that automatic naming remains consistent.
7. As a team administrator, I want an explicit subtitle preserved, so that
   my selected Chapter name is retained.
8. As a team member, I want the newly created Chapter pinned, so that the current
   Chapter remains discoverable through existing reads.
9. As a team member, I want a previous pinned Chapter unpinned when replaced,
   so that successful creation preserves the existing pin invariant.
10. As a team administrator, I want Chapter counts updated once per successful
    creation, so that Comic metadata remains accurate.
11. As a team member, I want Chapter creation to update Comic activity using
    the existing semantics, so that activity-based discovery remains correct.
12. As a team administrator, I want creation without a preset to leave the
    Chapter unassigned, so that team administration does not imply worker duties.
13. As a team administrator with eligible worker roles, I want explicit presets
    to create only the requested worker assignment, so that roles are predictable.
14. As a team owner, I want invalid or unavailable preset roles rejected under
    existing rules, so that this refactor does not expand privileges.
15. As a team member, I want Chapter creation recorded even without an assignment,
    so that creation history does not depend on worker participation.
16. As a team member, I want displacement of a prior pinned Chapter recorded,
    so that workflow history remains consistent with actual pin changes.
17. As a team member, I want first-Chapter creation to avoid fictitious unpin
    history, so that history represents only real changes.
18. As a team owner, I want failed Chapter creation to roll back counters, pins,
    assignments, and history, so that failed work leaves no partial result.
19. As a team owner, I want a failure creating a Comic's first Chapter to roll
    back Comic creation and Workset updates too, so that no incomplete Comic remains.
20. As a maintainer, I want existing transaction isolation and lock protection
    retained, so that concurrent work does not lose safeguards during consolidation.
21. As a client developer, I want existing requests, responses, and errors to
    remain compatible, so that no client update is needed.
22. As a test author, I want to verify complete results through the two existing
    use-case interfaces, so that tests survive internal reorganization.

## Implementation Decisions

- Consolidate the complete shared Chapter creation responsibility, rather than
  extracting only entry construction or assignment insertion and leaving the
  multi-operation protocol duplicated.
- The shared responsibility includes Chapter index allocation, identity and
  subtitle construction, Chapter insertion and pin transition, Comic chapter
  count and activity updates, optional worker assignment, and creation history.
- Place this behavior in an internal Chapter creation module within usecase.
  It performs domain transaction orchestration; it must not move into pure
  complex rules or into a repository adapter.
- Both existing public creation use cases consume that shared module. Preserve
  their input and output contracts and entity-specific authorization.
- The outer use case owns exactly one transaction through the existing Nucl
  mechanism. The shared module receives the existing context and uses Step
  operations. It must not start or commit an independent transaction.
- Comic creation retains responsibility for Workset authorization, Comic
  creation, and Workset index/count updates. Its first Chapter is created
  within the same transaction as those operations.
- Standalone Chapter creation retains authorization, loading and checking the
  existing Comic, and coordination of existing Chapters before shared mutation.
  Reuse loaded ownership and previous-pin information rather than opening a
  second read/authorization chain inside the shared module.
- Preserve current effective lock ordering, isolation requirements, and the
  transaction-local observation of the previous pinned Chapter. Do not move
  protected reads outside the transaction or weaken exclusion to simplify reuse.
- Shared pin mutation must handle the first-Chapter case with no previous pin
  and the existing-Comic case with a previous pin. Preserve removal of an old
  pin before inserting a new pinned Chapter where an old pin exists.
- Preserve successful first-Chapter and standalone creation behavior without
  inventing a caller-type abstraction, generic workflow engine, or new trait.
  Exact internal parameter packaging is an implementation choice; domain state
  and the existing transaction context must express the necessary facts.
- Create an assignment only for an explicit preset accepted by existing
  permission rules. Team administration alone creates no worker assignment.
- Record one ChapterCreated event in workflow history for each successful
  creation, including creation without an assignment. Preserve actor identity.
- When creation displaces a previously pinned Chapter, retain its ChapterUnpinned
  record and existing record ordering. Do not add a new ChapterPinned record to
  creation or fabricate an unpin record for first-Chapter creation.
- Keep all required writes in the caller-owned transaction. Propagate failures
  so that Chapter writes and any earlier Comic/Workset writes roll back together.
- Preserve existing errors and tracing behavior. Do not introduce external
  effects or a new error classification as part of this refactor.
- Remove superseded creation helpers once their complete responsibilities are
  covered. Retain unrelated pin-management, assignment-management, and workflow
  behavior in their existing owners.
- Reuse existing repository operations and RDB/Mock adapters. Keep SQL type-safe
  and preserve the schema, migrations, and HTTP contracts.
- Follow project naming, formatting, and module conventions, including the
  strict file-length limit. Module structure must serve the shared business
  responsibility rather than create a collection of forwarding functions.

## Testing Decisions

- The agreed primary test seam is the existing Comic-create and Chapter-create
  use-case interfaces. Exercise complete observable outcomes, not helper call
  counts, internal parameters, or the order of incidental implementation calls.
- Use existing Mock fixtures for ordinary use-case behavior, following the
  repository's established assertion and fixture conventions. No public
  testing-only interface or new dependency seam is required.
- Existing Comic creation/index/count tests, both paths' preset-assignment
  tests, Chapter administration tests, and workflow-history assertions are
  prior art. Extend them where needed rather than replacing them wholesale.
- Cover both creation paths with omitted presets, valid worker presets, denied
  creation, and invalid or unavailable preset roles. Assert assignments and
  creation history together for the no-preset case.
- Assert Chapter identity, parent linkage, allocated index, explicit/default
  subtitle, pin state, Comic count, and activity through existing observable
  results and established fixtures. Avoid timing-sensitive activity assertions.
- Cover first-Chapter creation and existing-Comic creation with and without a
  previous pinned Chapter. Verify actual pin state and exact relevant history
  payloads, actor attribution, and absence of extra creation/pin records.
- Prove rollback after writes have begun, including a failure during a required
  late assignment or history write. An authorization denial or missing-parent
  failure before writes is insufficient evidence of transaction atomicity.
- Verify that failed standalone creation restores previous pin state and leaves
  Chapter counts, activity, assignments, and history unchanged; verify that
  failed first-Chapter creation also leaves no new Comic or Workset counter/index
  changes from the failed transaction.
- Use the existing PostgreSQL test infrastructure to verify transaction rollback
  and relevant pin/index constraints. Keep existing concurrency regressions and
  add a focused case where necessary to protect affected creation behavior.
- Establish expected outcomes independently of the shared implementation. A
  comparison between the two callers alone is not proof of correctness.
- Keep SQL in Rust tests within the typed Diesel query DSL. Use existing
  disposable test fixtures; never target production or change startup migrations.
- Run focused use-case tests and applicable feature-gated PostgreSQL tests, then
  the full workspace suite at the end. After Rust edits, run `just fmt-check`,
  `just check`, and `just clippy` with the repository's exact command rules.
- Any implementation commit must pass the full pre-commit CI chain without
  bypassing hooks. Report actual validation results and any environmental gaps.

Acceptance requires both callers to use one shared creation implementation,
preserved permissions and pin/history behavior, demonstrated rollback of the
complete outer transaction, removal of obsolete duplicated orchestration, and
passing applicable tests and canonical checks.

## Out of Scope

- Changing team administration, worker role eligibility, or preset semantics.
- Changing public request/response contracts or introducing creation idempotency.
- Changing workflow event payloads, adding new history events, or altering the
  standalone pin-management operation.
- Redesigning repository interfaces, queries, locks, isolation, or retry policy.
- Introducing a general transaction framework, new port, or generic creation engine.
- Schema changes, database migrations, generated-artifact edits, or linter changes.
- Reworking the completed Unit sequence fix or implementing response hydration,
  the third architecture-review item.
- Creating another branch for this item; the fixes belong together on the
  existing fix/architecture-review branch.
- Implementing Rust changes, dispatching implementation sub-agents, committing,
  deploying, or releasing as part of this specification-only request.

## Further Notes

The user accepted the explanation and requested this specification. The testing
scope follows that explanation: existing creation use cases and real database
transaction verification. The subsequent implement command authorized this work
and sub-agent dispatch on the existing fix/architecture-review branch.

No domain glossary or ADR directory was found during this review. Terminology
and constraints follow the active Chapter administration documentation and
the existing Comic, Chapter, assignment, and workflow-history behavior.

Issue-tracker and triage-label configuration remains absent. Run
`/setup-matt-pocock-skills` before publishing through the configured tracker.
The to-spec workflow requires the ready-for-agent triage label when published;
publishing alone does not authorize starting implementation.

This spec does not claim that implementation validation has been performed.
