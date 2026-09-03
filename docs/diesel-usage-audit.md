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

### P0: Chapter Unit search is an N+1 query path

Search first loads every Page and then issues one Unit query per Page, with 20
queries started concurrently per batch. At the Chapter limit this is one Page
query plus as many as 200 Unit queries, and can transfer about 20,000 complete
Unit rows before Rust filters visibility and text and enforces the 100-match
limit.

Evidence:

- [`src/usecase/unit.rs`](../src/usecase/unit.rs#L175)

Repair research, reviewed 2026-09-03:

- Do not force this into one SQL statement. Result order is defined by Page
  index and the Unit `f_next_id` linked list, including hidden nodes that join
  two visible nodes. Diesel 2.3 has no typed recursive-CTE builder in the
  current dependency set. Raw `sql_query` would weaken compile-time query
  checking, while adding a second persisted order would create dual-source
  consistency and write-amplification risks.
- Add one transaction-scoped `SearchChapterUnitInfos` repository operation.
  Within a short repeatable-read snapshot, its first typed query joins Page and
  Unit, scopes by Chapter, requires `f_hidden_at IS NULL`, applies PostgreSQL
  `strpos(selected_text, phrase) > 0`, and uses `LIMIT 101`. It should select
  `(page_index, UnitSearchInfoRow)`: the purpose-specific Unit projection holds
  only fields needed by `UnitInfoView`, while Page index remains adapter-only.
- Zero matches return immediately. At 101 matches the use case preserves the
  existing Args error and returns no partial data. For 1--100 matches, a second
  typed query loads only `(page_id, id, next_id)` for the distinct matching
  Pages. The adapter validates those complete chains, derives Unit ranks, and
  sorts candidates by `(page_index, unit_rank)` before returning them.
- Keep phrase normalization and authorization in the use case. Run only the
  two Unit reads inside repeatable read; they need one coherent snapshot but no
  row locks. A phrase containing a zero byte must return an authorized empty
  result without binding it as PostgreSQL `text`, because stored PostgreSQL text
  cannot contain that byte.

Expected result: the search-specific data path falls from one Page query plus
up to 200 Unit queries to one or two Unit queries. It transfers at most 101
response projections plus minimal link rows for at most 100 matching Pages,
and it removes the 20-connection fan-out. Including the unchanged access
checks, the normal request uses four SQL data statements for a Team member or
five for a Chapter-only assignee.

This preserves all valid-data business behavior: Unicode-trimmed non-empty
phrases, literal and case-sensitive matching, selected-field-only matching,
hidden-row exclusion, Page/Unit order, exactly-100 success, 101-match failure,
and the existing response shape. One internal behavior needs an explicit test:
a corrupt chain on an unrelated nonmatching Page would no longer poison the
search. If that diagnostic behavior is considered part of the contract, the
second query must load minimal links for every Chapter Page instead.

Do not add a text-search index initially. Single-character searches are a
supported business case, and a trigram index does not remove that scan. The
existing Page-Chapter and Unit-Page indexes provide the join path. Compare
`EXPLAIN (ANALYZE, BUFFERS)` on 200-by-100 and tombstone-heavy fixtures first;
only then consider a partial visible-Unit index on `f_page_id`.

Implementation verification must cover both text parts, literal `%`, `_`, and
backslash characters, case sensitivity, a hidden node between visible nodes,
reordered Units, 0/100/101 matches, authorization, the zero-byte case, and the
chosen unrelated-corruption behavior. Add RDB query tests for the typed
projection and order reconstruction; the existing HTTP test already protects
the 100/101 public boundary.

### P0: Hierarchical deletion composes single-entity deletion paths

Team, Workset, Comic, Chapter, and Termbase cascades list descendants and then
invoke their single-entity deletion routines one by one. This repeats reads and
locks already performed by the parent cascade. Child deletion also updates
aggregate counters and timestamps on ancestors that will be deleted in the
same transaction.

Evidence:

- [`src/usecase/team/delete.rs`](../src/usecase/team/delete.rs#L135)
- [`src/usecase/workset.rs`](../src/usecase/workset.rs#L275)
- [`src/usecase/comic.rs`](../src/usecase/comic.rs#L353)
- [`src/usecase/chapter/delete.rs`](../src/usecase/chapter/delete.rs#L81)
- [`src/usecase/termbase.rs`](../src/usecase/termbase.rs#L369)

Action: introduce typed subtree-deletion operations that delete each dependent
table in batches and omit writes to doomed ancestors. Preserve archive rules,
object-deletion tasks, foreign-key ordering, permissions, and all observable
business effects; database `ON DELETE CASCADE` is not a sufficient replacement.

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

### P1: Database and request concurrency limits are not coordinated

The RDB pool uses builder defaults: it has no explicit application capacity or
acquisition timeout. HTTP has a global request-rate limit but no in-flight
request limit, timeout, or load shedding. The Unit search fan-out can therefore
consume the shared pool while other requests and background actors wait.

Evidence:

- [`poprako-rdb-core/src/rdb.rs`](../poprako-rdb-core/src/rdb.rs#L92)
- [`src/api/http/middleware/rate_limit.rs`](../src/api/http/middleware/rate_limit.rs#L15)

Action: configure pool capacity and acquisition timeout from deployment
settings, bound in-flight HTTP work relative to that capacity, and reserve
capacity for background consumers. Keep serialization failures as retryable
conflicts; do not weaken transaction isolation to avoid contention.

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
