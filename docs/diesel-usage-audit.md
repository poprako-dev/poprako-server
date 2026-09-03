# Diesel Usage Audit

This is the active audit record for production Diesel access and the use cases
that determine its query shape. Update or remove resolved findings so the
document continues to describe the checked-in implementation.

- Reviewed: 2026-09-03
- Repository revision: `c9982de6`
- Scope: 55 production Diesel-related Rust files, approximately 221 query
  execution sites, 56 coordinated transaction paths, and 35 explicit
  `FOR UPDATE` sites
- Method: static call-path review; tests and generated `schema.rs` were not
  treated as production query paths

Runtime query plans must still be verified against representative data with
`EXPLAIN (ANALYZE, BUFFERS)` before adding or changing indexes.

## Priority findings

### Resolved: Chapter import Create/Delete writes are batched

`ApplyUnitEdits` now inserts all created Units for a Page in one statement and
hides its exact deleted ID set in one statement. Created rows carry their final
successor when inserted, so only changed persisted predecessors need a later
update. Overwrite import also reuses the locked Unit order after marking its
visible entries hidden instead of loading the same order again.

Unit-order locking is keyset-chunked at 512 minimal projection rows per query.
The complete chain is still reconstructed and validated in Rust, preserving
the existing corrupt-chain behavior without returning a whole Page history in
one database result. Post-write counter calculation now selects only the three
required fields from visible Units; the Unicode-aware text rule remains in the
Rust model instead of being reimplemented in SQL.

Evidence:

- [`src/usecase/chapter_port/import.rs`](../src/usecase/chapter_port/import.rs#L85)
- [`src/part_impl/repo/rdb_impl/unit/edit.rs`](../src/part_impl/repo/rdb_impl/unit/edit.rs#L267)
- [`src/value/chapter_port.rs`](../src/value/chapter_port.rs#L7)
- [`src/value/unit.rs`](../src/value/unit.rs#L6)

Remaining bounded work: heterogeneous `Save` edits and changed existing
successors remain single-row updates. Each public edit batch is capped at 100,
while a Chapter import creates no heterogeneous saves and changes at most one
existing predecessor per imported Page. Page counter writes also remain one
per changed Page. Further batching requires representative query measurements
before adding more complex typed SQL.

### Resolved: Chapter Unit search no longer fans out by Page

Search now uses a bounded, typed three-stage read instead of introducing a
search-specific god object:

1. PostgreSQL applies Chapter scope, visibility, selected-field literal
   matching, and `LIMIT 101`, returning Unit IDs only.
2. For 1--100 matches, the existing `UnitInfo` projection is loaded for those
   IDs. This deliberately reuses the established domain projection rather than
   creating another wide `SearchInfo` type.
3. Only the matching Pages' `(page_id, id, next_id)` links are loaded. The
   existing chain validator derives Unit rank, and results are sorted by Page
   index and chain order.

The overflow path executes one Unit query; a successful non-empty search
executes three, independent of Chapter Page count. The former 20-query
concurrency fan-out and worst-case 201 Unit/Page queries are gone. All three
reads share a short repeatable-read transaction; phrase validation and
authorization remain outside it. NUL remains an authorized empty result
because PostgreSQL text cannot contain or bind it.

No migration or speculative index was added. Single-character matching is a
business requirement, so representative `EXPLAIN (ANALYZE, BUFFERS)` evidence
is still required before considering a partial visible-Unit index.

Verified behavior includes both text fields, literal `%`, `_`, and backslash,
case sensitivity, Unicode trimming, hidden chain nodes, stable Page/Unit order,
NUL, and the exact 100/101 boundary.

Evidence:

- [`src/usecase/unit.rs`](../src/usecase/unit.rs#L142)
- [`src/part_impl/repo/rdb_impl/unit/sequence.rs`](../src/part_impl/repo/rdb_impl/unit/sequence.rs#L227)
- [`src/usecase/unit/tests/search.rs`](../src/usecase/unit/tests/search.rs#L1)
- [`tests/integration-tests/src/suites/it_05_unit_save_order_count.ts`](../tests/integration-tests/src/suites/it_05_unit_save_order_count.ts#L102)

### Resolved P0: Hierarchical deletion

Team, Workset, and Comic deletion now atomically marks the selected hierarchy
with one timestamp. Marked Team, Workset, Comic, and Chapter rows are excluded
from normal reads and guarded mutations immediately. Direct Chapter deletion
remains synchronous because Chapter is the cleanup unit and its Page count is
frozen while deletion is in progress.

Two scheduler workers repeatedly claim eligible rows with `FOR UPDATE SKIP
LOCKED`. Claims are dependency ordered: Chapter, then childless Comic, then
childless Workset, then childless Team. Each transaction deletes only one
claimed level and its direct dependants. Object deletion tasks are inserted in
the same transaction before the relational rows disappear, so a rollback leaves
the target available for a later sweep. Prom itself keeps its existing unordered
semantics.

The shared RDB pool is capped at four connections for the 2C2G production host;
the deletion scheduler uses two workers. Permanent-failure quarantine remains a
focused FIXME. No ordered task table or application-side copy of an entire
hierarchy was introduced.

The repository contract and every production execution boundary were also
audited in reverse: all declared domain `Oper`s have direct use-case consumers,
and no HTTP, scheduler, effect, or Prom business handler executes one outside
`src/usecase`. Thirteen obsolete operations and their RDB/mock implementations
were removed. Domain-event and deferred-task business orchestration now lives in
business-domain use cases; their actors only provide queueing, dispatch, and
retry lifecycles. This caller-independent boundary is codified in
[`usecase-boundaries`](../.agents/skills/usecase-boundaries/SKILL.md).

Tombstone follow-up guards cover adjacent lifecycle paths: user deletion
batch-removes memberships even when their teams are already hidden; archive
snapshot and commit both require an active Comic; and normal Page identity
reads reject Pages whose Chapter is tombstoned, preventing late upload
confirmation.

Evidence:

- [`src/part_impl/repo/rdb_impl/subtree_delete.rs`](../src/part_impl/repo/rdb_impl/subtree_delete.rs)
- [`src/part_impl/repo/rdb_impl/subtree_delete/mark.rs`](../src/part_impl/repo/rdb_impl/subtree_delete/mark.rs)
- [`src/part_impl/repo/rdb_impl/subtree_delete/sweep.rs`](../src/part_impl/repo/rdb_impl/subtree_delete/sweep.rs)
- [`src/usecase/subtree_delete.rs`](../src/usecase/subtree_delete.rs)
- [`src/usecase/internal/subtree_delete.rs`](../src/usecase/internal/subtree_delete.rs)
- [`src/extra/sched/subtree_delete.rs`](../src/extra/sched/subtree_delete.rs)
- [`src/part_impl/repo/rdb_impl/subtree_delete/tests.rs`](../src/part_impl/repo/rdb_impl/subtree_delete/tests.rs)

### P0: Public list size is unbounded and archive export materializes payloads

Fifteen public list inputs expose raw `u32` `offset` and `limit` values without
a shared maximum. All use offset pagination. Several time-ordered queries lack
an ID tie-breaker, so equal timestamps can produce unstable pages.

Archive export limits the request to 12 month labels but does not limit the
number or byte size of archive JSON rows. Selecting non-contiguous months reads
the complete range between the first and last month and filters the gaps in
Rust.

Evidence:

- [`src/data/instr/comic.rs`](../src/data/instr/comic.rs#L94)
- [`src/part_impl/repo/rdb_impl/comic_archive/payload.rs`](../src/part_impl/repo/rdb_impl/comic_archive/payload.rs#L44)
- [`src/usecase/comic_archive.rs`](../src/usecase/comic_archive.rs#L79)

Action: establish a shared default and hard maximum for ordinary lists, add a
unique ordering suffix, and migrate high-volume lists to keyset pagination.
Archive export must remain complete: stream or chunk it rather than truncate
it, and push the selected month intervals into the typed SQL predicate.

### P1: Comic stage filtering materializes global IDs

Stage-filtered Comic listing first queries distinct matching Comic IDs from
pinned Chapters across the whole database. It then sends those IDs back in a
second query that finally applies the requested Workset scope.

Evidence:

- [`src/part_impl/repo/rdb_impl/comic/stage_filter.rs`](../src/part_impl/repo/rdb_impl/comic/stage_filter.rs#L41)
- [`src/part_impl/repo/rdb_impl/comic/step_impl.rs`](../src/part_impl/repo/rdb_impl/comic/step_impl.rs#L84)

Action: express the stage condition as a typed, correlated `EXISTS` predicate
inside the scoped Comic query.

### P1: Request concurrency is not coordinated with the database pool

The RDB pool is now explicitly capped at four connections for the 2C2G
production host. HTTP still has a global request-rate limit but no in-flight
request limit, timeout, or load shedding, so request concurrency can still
occupy all four connections while background actors wait.

Evidence:

- [`poprako-rdb-core/src/rdb.rs`](../poprako-rdb-core/src/rdb.rs#L92)
- [`src/api/http/middleware/rate_limit.rs`](../src/api/http/middleware/rate_limit.rs#L15)

Action: add an acquisition timeout, bound in-flight HTTP database work below
the four-connection capacity, and reserve capacity for background consumers.
Keep serialization failures as retryable conflicts; do not weaken transaction
isolation to avoid contention.

### P1: Object-task polling performs continuous maintenance writes

The globally serial object actor calls `reset_tasks` before every claim.
`reset_tasks` executes three maintenance updates, and an idle actor then runs a
claim select every five seconds. This is four SQL statements every five seconds,
or about 48 statements per minute per application instance, even with no work.

Evidence:

- [`poprako-obj-dept/src/actor.rs`](../poprako-obj-dept/src/actor.rs#L174)
- [`poprako-obj-dept-macro/src/rdb_obj_prom.rs`](../poprako-obj-dept-macro/src/rdb_obj_prom.rs#L187)

Action: run recovery maintenance on an independent, lower-frequency interval
and claim work atomically with a typed `UPDATE ... RETURNING` operation. Retain
lease fencing and bounded processing concurrency.

### P1: Other bounded batches still use repeated single-row operations

Term import bulk-inserts new Terms but updates as many as 200 existing Terms
one by one. User deletion loads and locks the complete roster of every Team the
user belongs to, then deletes memberships individually. Assignment and Member
role changes similarly load full collections to evaluate administrator
retention.

Evidence:

- [`src/part_impl/repo/rdb_impl/term.rs`](../src/part_impl/repo/rdb_impl/term.rs#L375)
- [`src/usecase/user/delete.rs`](../src/usecase/user/delete.rs#L72)
- [`src/usecase/assignment/update_roles.rs`](../src/usecase/assignment/update_roles.rs#L58)
- [`src/usecase/member.rs`](../src/usecase/member.rs#L340)

Action: use typed bulk upsert/delete operations and purpose-specific aggregate
queries that return only offending Team or Chapter IDs. The invariant that a
Team or Chapter retains an administrator must remain transactionally safe.

## Secondary improvements

- Add purpose-specific model projections for cascade IDs, Page object IDs,
  import-target lookup, and latest-Chapter selection. Repository operations
  must continue to return models or projections, not request/response `View` or
  `Val` DTOs.
- Return only IDs from create operations whose callers consume only IDs,
  instead of returning complete `Info` rows.
- Push Unit counters, administrator existence checks, exact Termbase-name
  lookup, and latest-Chapter selection into typed aggregate, existence, or
  `LIMIT 1` queries.
- Give multi-row `FOR UPDATE` queries a deterministic lock order. Configure
  database statement and lock timeouts so large cascades cannot wait forever.
- Comic update reads the same Comic in the use case and again inside the
  adapter. Reuse one transactionally valid projection containing the index
  needed to compose the title.

## Business and transaction safeguards

- Serializable or repeatable-read transactions protect business invariants;
  optimize work inside them before considering a weaker isolation level.
- Comic archive preparation serializes a potentially large JSON payload while
  descendant rows remain locked. Moving it outside the transaction without a
  version check or two-phase protocol would break snapshot consistency.
- Chapter translation export correctly loads all Pages and Units in batches.
  If the exported document must represent one database version, those reads
  should use a short repeatable-read snapshot rather than independent pooled
  connections.
- Destructive Team, Workset, Comic, and Chapter permission checks currently
  occur before their deletion transaction. A cascade rewrite must revalidate
  the authorization evidence transactionally instead of moving more checks
  outside the transaction.
- Complete import and export operations must not be truncated or made
  partially successful for performance reasons.

## Existing patterns to retain

- Include hydration batches foreign-key IDs with `eq_any` rather than issuing
  one query per result:
  [`src/part_impl/repo/rdb_impl/incl/macros.rs`](../src/part_impl/repo/rdb_impl/incl/macros.rs#L20).
- Chapter translation export fetches all Units for all Page IDs in one query:
  [`src/usecase/chapter_port/export.rs`](../src/usecase/chapter_port/export.rs#L96).
- Edited-diff Page detection uses a correlated `EXISTS` predicate:
  [`src/part_impl/repo/rdb_impl/page/step_impl.rs`](../src/part_impl/repo/rdb_impl/page/step_impl.rs#L141).
- System-mail read marking validates a projected batch and updates the batch in
  one statement:
  [`src/part_impl/repo/rdb_impl/system_mail.rs`](../src/part_impl/repo/rdb_impl/system_mail.rs#L103).

## Recommended implementation order

1. Batch Chapter import and Unit edits.
2. Replace Chapter Unit search fan-out and establish concurrency limits.
3. Implement purpose-built subtree deletion.
4. Bound ordinary lists and rewrite archive and stage filters.
5. Batch Term and membership writes, then introduce smaller projections and
   database aggregates.

## Usecase naming lint

No violations were found. A total of 265 public functions were scanned in the
usecase and complex layers; all names follow the current convention.
