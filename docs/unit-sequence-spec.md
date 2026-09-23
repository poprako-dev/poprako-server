# Spec: Shared Unit sequence reconstruction

Status: Implementation authorized on 2026-09-23; not published to an issue tracker.

Execution: The user explicitly authorized implementation and sub-agent dispatch
through the implement workflow on 2026-09-23.

## Problem Statement

Maintainers currently have to preserve the same persisted Unit chain rules in
two separate repository adapters. The RDB adapter reconstructs and validates
chains using indexed lookups; the Mock adapter independently implements the
same responsibility using repeated scans. Both already share pure edit planning.

This distribution makes changes harder to reason about and allows the behavior
exercised by Mock-based tests to drift from production. Pure reconstruction tests
also live under a storage adapter even though the rules do not require storage.
No existing correctness defect has been demonstrated. This is a bounded
refactor for locality, test ownership, and reuse.

## Solution

Give persisted Unit chain reconstruction one pure implementation in the Unit
domain logic, alongside the existing edit-planning responsibility. Both repository
adapters use it while retaining their own record retrieval and persistence.

Preserve observable ordering, tombstone behavior, corruption classification,
editing behavior, and transaction semantics. Keep the existing distinction
between reconstructing unordered records and verifying the already-ordered
input accepted by edit planning.

## User Stories

1. As a maintainer, I want one implementation of persisted chain reconstruction,
   so that a rule correction applies to both repository adapters.
2. As a maintainer, I want pure sequence rules owned by the Unit domain module,
   so that understanding those rules does not require navigating storage code.
3. As a translator, I want Units returned in their linked-list order regardless
   of database retrieval order, so that my reading and editing sequence is stable.
4. As a translator, I want moving a Unit to retain its established semantics,
   so that this internal refactor does not change editing behavior.
5. As a translator, I want hidden Units retained in the complete stored chain,
   so that deletion and later restoration preserve valid ordering.
6. As a reader, I want hidden Units omitted from visible results at the existing
   point in processing, so that tombstones do not appear as visible content.
7. As a maintainer, I want empty chains accepted, so that empty pages remain valid.
8. As a maintainer, I want a single terminal Unit accepted, so that single-Unit
   pages retain their behavior.
9. As a maintainer, I want duplicate Unit IDs rejected, so that ambiguous identity
   cannot silently produce an apparently valid chain.
10. As a maintainer, I want missing successors rejected, so that incomplete
    persisted chains cannot silently truncate results.
11. As a maintainer, I want competing predecessors rejected, so that traversal
    does not silently discard a branch.
12. As a maintainer, I want self-links and cycles rejected, so that corrupt
    structures cannot produce nonterminating traversal or misleading order.
13. As a maintainer, I want disconnected chains and unreachable nodes rejected,
    so that successful reconstruction accounts for every input record.
14. As a maintainer, I want edit planning to keep checking its ordered-input
    contract, so that an incorrectly assembled sequence still fails explicitly.
15. As a test author, I want graph-rule tests to run without a database,
    so that those rules can be verified directly and cheaply.
16. As a test author, I want expected results specified independently of the
    implementation, so that sharing logic between adapters does not hide defects.
17. As a maintainer, I want database-specific query and persistence tests retained,
    so that shared pure logic does not replace verification of the RDB adapter.
18. As a maintainer, I want linear expected reconstruction work retained,
    so that the production path does not inherit the Mock algorithm's repeated scans.
19. As a client developer, I want existing response shapes and error behavior
    preserved, so that no client changes are required.
20. As the project owner, I want implementation to wait for my next command,
    so that publishing or reviewing this spec does not dispatch agents or modify Rust.

## Implementation Decisions

- Scope is limited to consolidating persisted-chain reconstruction and its test
  ownership. Redesigning the edit-planning interface is not part of this work.
- Place the shared pure responsibility in the existing Unit complex module's
  organization. Follow the project's module-level function convention; do not
  introduce an empty holder type or a new storage abstraction.
- Reuse the existing reconstruction interface shape where practical. Do not
  require full Unit content merely to order minimal persisted links.
- Route both RDB and Mock reconstruction call sites through the shared
  implementation, including complete-order reads, applicable detail reads, and
  chapter-search ranking. Remove the replaced adapter-local algorithms.
- Prefer the existing indexed RDB algorithm as the consolidation baseline.
  Preserve expected linear time and linear auxiliary storage. Do not introduce
  repeated full-array scans into the production reconstruction path.
- A successful reconstruction must include every input record exactly once,
  follow each persisted successor, and terminate at the sole tail. Empty input
  remains valid. Invalid non-empty input must be rejected.
- Preserve each record's identity, successor, hidden state, and other payload
  fields. Reconstruction changes record ordering; it does not repair persisted
  links or modify domain content.
- Hidden Units participate in chain reconstruction. Visibility filtering remains
  at its established call sites after the complete sequence is handled.
- Preserve the edit planner's existing ordered-input checks. Recovering order
  from unordered records and verifying supplied order are distinct contracts;
  consolidating reconstruction does not justify removing planner validation.
- Keep corruption classified as an unrecoverable persisted-data error. Preserve
  existing caller-visible error behavior and avoid new logging or error categories.
- Keep record fetching, query batching, database mapping, search selection,
  mutations, transaction ownership, and isolation choices with their existing owners.
- Existing repository interfaces and their RDB and Mock adapters remain the
  storage seam. No new trait, dependency injection mechanism, or validated-chain
  representation is required.
- No schema, migration, HTTP contract, response DTO, or client change is required.
- Preserve typed Diesel queries and project Rust conventions. Keep Rust files
  strictly below 600 lines, using project module-splitting rules if needed.

## Testing Decisions

- Test externally observable behavior at each existing interface, not the
  internal maps, temporary indexes, swap sequence, or exact helper call graph.
- Prefer existing Unit use-case tests for user-visible ordering, edits, counts,
  restoration, and visibility. These remain the highest behavior seam.
- Use the shared pure reconstruction interface for malformed persisted graphs
  that normal use cases cannot create. This is an internal test seam, not a new
  public testing-only interface or storage port.
- Rehome the existing RDB-local shuffled-chain and corruption cases into tests
  beside the shared pure implementation. Remove obsolete algorithm-local tests
  once equivalent shared coverage exists; retain distinct adapter regressions.
- Cover empty input, a terminal singleton, ordered and shuffled valid chains,
  duplicate IDs, missing successors, competing predecessors, self-links, cycles,
  multiple heads, disconnected chains, and a valid chain plus an unreachable cycle.
- Cover tombstones at different positions in a complete chain. Assert that
  reconstruction preserves records and metadata while ordering them correctly.
- Keep the edit planner's tests for mismatched ordered input and duplicate IDs,
  as well as existing create, delete, restore, move, and count-limit scenarios.
- Retain focused adapter tests that verify real retrieval, mapping, and integration
  with shared reconstruction. Do not duplicate the entire pure graph matrix for
  each adapter.
- Retain relevant PostgreSQL query, persistence, constraint, concurrency, and
  rollback tests. Mock tests cannot establish those properties. Run the applicable
  feature-gated tests with their existing fixtures and required features enabled.
- Use explicit expected sequences and error classifications as the oracle.
  Agreement between two adapters sharing the same implementation is insufficient.
- Do not add timing-sensitive tests or a benchmark project. Assess algorithmic
  complexity from the implementation and retain applicable long-chain regressions.
- After implementation, run targeted sequence, planner, adapter, and Unit use-case
  tests, together with `just fmt-check`, `just check`, and `just clippy` from the
  repository root. Follow the project's exact command and environment rules.
- Record which tests ran, including any database-dependent tests that could not
  run. A default-feature test pass does not establish PostgreSQL coverage.

Acceptance requires both adapters to consume the single reconstruction
implementation, the replaced algorithms to be removed, the ordered-input planner
contract to remain intact, and applicable behavior tests and canonical Rust
checks to pass.

## Out of Scope

- Introducing a validated-chain type or redesigning edit-planning inputs.
- Combining save and transform orchestration, counters, receipts, or workflow effects.
- Changing Unit edit normalization, limits, identity resolution, or replay semantics.
- Repairing corrupt stored chains or changing their error classification.
- Changing SQL queries, pagination, database constraints, or transaction isolation.
- Refactoring chapter creation or response hydration from the other review candidates.
- Introducing a generic graph framework, new repository seam, or benchmark suite.
- Schema changes, migrations, generated-artifact edits, linter changes, deployment,
  or release preparation.
- Starting implementation, dispatching implementation sub-agents, or creating a
  commit before the user's subsequent command.

## Further Notes

This spec synthesizes the architecture discussion's limited consolidation option.
The broader validated-chain design remains deliberately unselected.

The proposed testing seams are the existing Unit use-case interface, the shared
pure reconstruction interface, and existing repository adapter integration tests.
The subsequent implementation command authorized this specification and its
testing scope.

Issue-tracker and triage-label configuration was not found. Run
`/setup-matt-pocock-skills` before publishing this spec through the configured
tracker. The to-spec workflow calls for the `ready-for-agent` triage label when
published; that label must not override the explicit requirement to await the
user's implementation command.

Implementation validation results are recorded separately from this specification.
