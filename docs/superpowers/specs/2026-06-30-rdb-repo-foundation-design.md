# RdbRepo Foundation and Existing Tables Design

## Goal

Build the first production RDB repository slice for the active ports-and-steps
architecture. This slice covers the repository foundation plus the tables that
already have local migrations and previous Diesel query/entity references:
`user`, `team`, `member`, `member_invitation`, `system_mail`, `workset`,
`local_message` as the prom store, and `comic`.

## Scope

This design covers:

- `RdbRepo`, `RdbRepoTransactional`, and the RDB transaction context.
- `RdbDrive`, the separate RDB transaction driver.
- Diesel entity modules for the first-scope tables.
- `Execute<S>` and `Advance<S, C>` implementations for first-scope repo steps.
- `Prom<C>` / `PromTransactional<C>` backed by the local-message table.
- Migration and schema regeneration workflow for first-scope schema fixes.
- A structure that later supports local PostgreSQL integration tests.

This design does not cover:

- New-table domains that are not part of the first slice:
  `chapter`, `page`, `unit`, `assignment`, `assignment_invitation`,
  `announcement`, and `comment`.
- HTTP wiring.
- Full local database integration tests. The implementation must stay ready for
  them, but those tests will be added later.

## Sources Of Truth

The database schema must be aligned with the original RDB design and previous
Diesel entity implementation, not with active `model` structs.

Use these sources in this order:

1. `references/poprako-s/migrations/`
2. `references/poprako-s/internal/infra/repo/entity/`
3. Deleted previous Rust query/entity files from commit `ba0916c^`, especially
   `src/infra/repo/entity/*.rs.bak` and `src/infra/repo/*.rs.bak`
4. Active `part::repo::step` traits, only for the Rust port surface and step
   outputs

`model` is the internal application data carrier. `data` is the external data
exchange layer. Neither one is a database schema source. RDB entity structs own
their own shape and convert at the repository boundary.

## Architecture

`part_impl::rdb_repo` is the production adapter for `part::repo` and `part::prom`.
It must remain a thin adapter around Diesel entities and query functions. It
must not introduce business rules that belong in `complex` or `usecase`.

`part_impl::rdb_drive` is the production transaction driver. It is a separate
type from `RdbRepo` because application wiring keeps the drive and repo as
separate fields. `RdbDrive` and `RdbRepo` must share the same Diesel pool, but
neither type should own or wrap the other.

Recommended module layout:

```text
src/part_impl/rdb_drive.rs
src/part_impl/rdb_repo.rs
src/part_impl/rdb_repo/error.rs
src/part_impl/rdb_repo/entity.rs
src/part_impl/rdb_repo/entity/user.rs
src/part_impl/rdb_repo/entity/team.rs
src/part_impl/rdb_repo/entity/member.rs
src/part_impl/rdb_repo/entity/member_invitation.rs
src/part_impl/rdb_repo/entity/system_mail.rs
src/part_impl/rdb_repo/entity/workset.rs
src/part_impl/rdb_repo/entity/local_message.rs
src/part_impl/rdb_repo/entity/comic.rs
src/part_impl/rdb_repo/user.rs
src/part_impl/rdb_repo/team.rs
src/part_impl/rdb_repo/member.rs
src/part_impl/rdb_repo/member_invitation.rs
src/part_impl/rdb_repo/system_mail.rs
src/part_impl/rdb_repo/workset.rs
src/part_impl/rdb_repo/local_message.rs
src/part_impl/rdb_repo/comic.rs
```

The `rdb_drive` module is responsible for transaction driving. The `rdb_repo`
root module is responsible only for module declarations, pool construction, and
shared connection helpers for non-transactional execution. Per-domain files own
their own step implementations. Entity files own row structs and conversions.

## Transaction Model

`RdbRepo` holds a Diesel async PostgreSQL pool for non-transactional repository
and prom execution.

`RdbDrive` holds the same Diesel async PostgreSQL pool for transaction driving.
It should be constructed from the same pool clone as `RdbRepo`, or both should
be produced by a small factory that clones one pool into the two fields. The
important invariant is that production wiring can inject them as separate
values while they still target the same database.

`RdbRepoTransactional` should stay a small stateless handle, matching the
current mock implementation shape. It implements `Advance<S, RdbContext>` for
transactional steps.

`RdbContext` must carry the live transaction connection. All transactional
query functions must receive the connection through `&mut RdbContext`, never
from the pool directly. This keeps the generic `C` anchor meaningful:

- `impl UserRepo<RdbContext> for RdbRepo`
- `impl UserRepoTransactional<RdbContext> for RdbRepoTransactional`
- `impl Advance<GetInfoById<'_>, RdbContext> for RdbRepoTransactional`
- `impl Prom<RdbContext> for RdbRepo`
- `impl PromTransactional<RdbContext> for RdbRepoTransactional`
- `impl Drive<RdbContext> for RdbDrive`

Non-transactional `Execute<S>` implementations allocate one pooled connection
for one step. Transactional `Advance<S, RdbContext>` implementations must not
allocate a new connection.

`Drive<RdbContext> for RdbDrive` must begin a database transaction before calling
the closure, commit after `Ok`, and roll back after `Err`. Backend begin,
commit, and rollback errors map to the `DriveError::Backend` path.

## Entity Rules

Every SQL shape gets a precise entity. Do not reuse a wider entity just because
it compiles.

Required suffixes:

- `*Entry`: insert-only entity, normally `Insertable`.
- `*Row`: select/read entity, normally `Queryable` and `Selectable`.
- `*Aspect`: patch changeset, normally `AsChangeset`, using builder pattern;
  only `Some` fields are updated.
- `*Save`: full replacement update entity, used for model update semantics
  where every writable field is replaced.

If two queries need different columns, define two entity structs. Examples:

- A credential lookup must use a credential-specific row, not a full user row.
- Include population must use a row containing only fields needed for that
  include relation.
- A returning clause must return exactly the row needed by the step output.

Conversions from entity to model live next to the entity type. When conversion
can fail, use `TryFrom` and return `RootError`.

Role and workflow columns must be converted explicitly. Do not use unchecked
casts from raw database values into domain value types.

## Query Rules

Redundant queries are forbidden.

Use `RETURNING` when a write step needs the written row. For example, insert
steps that return `UserInfo`, `TeamInfo`, `WorksetInfo`, or `ComicInfo` should
insert with `RETURNING <ExactRow>::as_returning()` instead of insert plus select.

Steps returning `()` should not issue a follow-up select. They should check the
affected row count only when the step contract requires a not-found error.

Steps returning `Option<T>` should not map absence to an error.

Steps with `Excluded` in their name must use row-level locking. The lock belongs
to that step only, not to ordinary read steps.

List queries must apply filters, ordering, offset, and limit before include
population. Includes are populated after the main list query.

## Include Rules

Includes must be implemented as one main query plus one batch query per include
option that is present.

The batch query pattern is:

1. Collect the foreign keys from the already loaded main rows.
2. Sort and deduplicate the keys.
3. If the key set is empty, skip the include query.
4. Query the related table with `eq_any`.
5. Build a map keyed by ID.
6. Fill optional relation fields on the returned model values.

This intentionally mirrors the previous include strategy: `len(incl_opt)` batch
queries, not one query per row and not one giant join.

Nested includes are not part of this first slice except where an active
first-scope step explicitly requires them. If a nested include is required later,
it must be specified as its own batch include path with exact row structs.

## Error Handling

Diesel unique violations map to `RootError::Expected` with conflict semantics.

Diesel `NotFound` must not be propagated directly. Query functions must use
`.optional()?` and map `None` into the step-specific expected error where the
step output is non-optional.

Connection pool errors, invalid stored enum values, JSON serialization errors,
and unexpected Diesel errors map to `RootError::Unrecoverable`.

Error messages must be contextual enough to identify the failed RDB operation.
User-facing messages must use existing `trl(...)` keys where the project already
has one for that condition.

## Migration Workflow

Schema work must precede Rust code for each batch.

For each migration batch:

1. Inspect the corresponding original migration and previous entity/query code.
2. Add or edit the local migration files.
3. Run `just mgr-run`.
4. Run `just mgr-schema`.
5. Inspect `src/infra/repo/schema.rs`.
6. Only then implement or adjust Rust entity/query code.
7. Run `cargo fmt`.
8. Run targeted checks.
9. Run `cargo check`.

Do not manually edit `src/infra/repo/schema.rs` except through
`just mgr-schema`.

The first slice should avoid unrelated schema redesign. If the current local
schema differs from the original schema for a deliberate reason, document that
reason in the migration batch notes before writing Rust against it.

## First-Scope Domain Order

Implement first-scope domains in dependency order:

1. Foundation: pool, context, transaction driving, error conversion.
2. `user`: profile rows, credentials, avatar reservation, last-active touch.
3. `team`: profile rows, avatar reservation, workset index allocation.
4. `member`: role timestamp columns, user/team includes, membership lookup.
5. `member_invitation`: invitation code lookup, pending transition, invitor
   include.
6. `system_mail`: send, send batch, list by receiver, list by IDs, mark read.
7. `workset`: team-scoped list, update, delete, comic counters.
8. `local_message` as `prom`: append image intentions inside transactions.
9. `comic`: workset list, fuzzy title filtering, workset/team/creator includes,
   cover reservation, counters, last-active touch.

The first implementation plan must stop after this list. Later domains need a
separate spec or a clearly approved extension of this spec.

## Testing Strategy

During first implementation, compile-time checks and focused unit-level adapter
tests are acceptable. The design must not fake confidence with mock-only tests
for RDB behavior.

The RDB code must be structured so later local PostgreSQL integration tests can:

- Build a `RdbRepo` from a database URL.
- Build a `RdbDrive` from the same database URL or shared pool.
- Run migrations with `just mgr-run`.
- Reset or isolate test data.
- Exercise non-transactional `Execute<S>` steps.
- Exercise transactional `Drive<RdbContext>` plus `Advance<S, RdbContext>`.
- Verify rollback behavior.
- Verify include batch loading.
- Verify prom records commit with repo changes.

The later integration tests should connect to a local database. They are not
required in the first implementation plan unless explicitly requested.

## Subagent Execution Strategy

The later implementation plan should be executed by sequential subagents, not
parallel implementation subagents. These tasks share migrations, generated
schema, and repository modules; parallel editing would create conflicts.

Recommended sequential subagent slices:

1. Foundation and separate transaction driver bridge.
2. Existing-table schema audit and migration batch.
3. Entity modules for `user`, `team`, and `member`.
4. RDB repo implementations for `user`, `team`, and `member`.
5. Entity modules for `member_invitation`, `system_mail`, and `workset`.
6. RDB repo implementations for `member_invitation`, `system_mail`, and
   `workset`.
7. Prom/local-message entity and implementation.
8. Comic entity and implementation.
9. Final compile/style review and integration-test readiness review.

Each subagent must receive the exact task text, the relevant source paths, and
the constraints from this spec. Each task should be reviewed for spec compliance
first, then code quality.

## Acceptance Criteria

The first slice is complete when:

- `RdbDrive` implements `Drive<RdbContext>`.
- `RdbRepo` implements the first-scope repository and prom traits.
- `RdbDrive` and `RdbRepo` can be constructed from the same shared pool.
- First-scope Diesel entities use precise `Entry`, `Row`, `Aspect`, and `Save`
  types as needed.
- Writes that return rows use `RETURNING` instead of redundant follow-up
  selects.
- Includes use batch `eq_any` loading and do not query per row.
- Local migrations and generated Diesel schema are synchronized through
  `just mgr-*`.
- `cargo fmt` and `cargo check` pass.
- Any unimplemented later-domain repo traits are outside the first-scope plan
  and clearly not claimed as complete.
