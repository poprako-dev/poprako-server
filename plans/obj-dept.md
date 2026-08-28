# ObjDept Implementation Plan

Status: reviewed draft v3

Source contract: `specs/obj-dept.md`

## Safety record

The local PostgreSQL database was backed up before design with `pg_dump` in
custom archive format and verified with `pg_restore --list`.

```text
path: logs/db-backups/poprako-20260828-000434.dump
size: 79696 bytes
sha256: 5886328b61a3b6cf62b2b9713dbe1e00bf5b7cbe16efd49765a4ff1e9da8917e
```

Local inventory:

```text
database: db_poprako_r on PostgreSQL 18.3
page rows: 1
complete active page tuples: 1
verified page tuples: 1
highest page version: 1
t_page_image: absent
legacy image messages: 1 Completed, 0 unfinished
```

No migration is run against this database during design. Destructive migration
checks use only the disposable CI database permitted by repository policy.

## Stage 0: Lock the part API

Complete `poprako-obj-dept` without RDB dependencies.

Deliverables:

- make all four existing operation types real Orchestra operations;
- split their capability aggregates into business-side `ObjAccessDept<B>` and
  actor-side `ObjActorDept<B>`;
- expose opaque `ObjAccess<D>` and `ObjActorAccess<D>` wrappers without an
  accessor to the complete adapter;
- parameterize operation identity by a static binding `B`;
- keep `ObjKey { id, version }` as the complete logical key;
- keep `ObjSpec` centered on `ObjKeyRef`;
- make `ObjSlot` carry URL, required headers, and absolute expiry;
- define the smallest storage error classification needed by actor retry;
- prove the new key grammar does not use `hash`, `ext`, chapter id, or RDB;
- add an Image wrapper fixture without adding Image semantics to core.

API proof cases:

1. two binding markers compile against one storage adapter without a runtime
   kind;
2. operation payload serialization, where needed by adapters, does not contain
   the binding marker;
3. `GenObjSlot` uses the supplied absolute expiry;
4. too little remaining lifetime rejects the slot;
5. `DelObjs` treats an absent exact key as success.
6. a use-case fixture that receives `ObjAccess<D>` cannot satisfy a
   `GetObjMeta<B>` or `DelObjs<B>` operation bound.

Review gate:

- use cases can receive URL/slot capability without receiving the actor-owned
  physical delete capability through Harn composition.

## Stage 1: Extract the RDB-neutral base

Move only required RDB infrastructure into `poprako-rdb-impl` before actor
implementation:

- `RdbCore`;
- `RdbContext`;
- generic transaction coordination without application error dependency;
- `RdbError`;
- local-message entity and raw lease operations;
- shared numeric conversions proven not to be application policy.

Keep server migrations, projections, application repo operations, and
`BaseError` in `poprako-server`. Add the application conversion from
`RdbError` at the existing adapter boundary.

Validation:

- dependency graph has no server back-edge;
- current repository and Prom behavior compiles through the moved types;
- focused transaction rollback and lease tests pass before ObjDept code is
  added.

## Stage 2: Prove the RDB macro

Add `poprako-rdb-impl-macro` and a fake standardized table fixture.

The macro contract is fixed before Page use:

```rust
#[obj_dept(Image, t_fake_obj)]
pub struct Fake;
```

Deliverables:

- annotated type is the binding marker;
- macro implements `RdbObjBinding` only for that local type;
- macro fixes `type Dept = Image` on the binding;
- macro emits concrete standardized Diesel query methods;
- topic is mechanically `obj_dept:t_fake_obj`;
- `RdbObjRepo<B>` owns generic Orchestra implementations;
- no identifier-concatenated hidden type;
- no foreign-trait/foreign-self implementation;
- compile-fail tests for missing or wrongly typed standardized columns;
- compile proof for two bindings with no central match;
- compile-fail proof that actor composition cannot replace a binding's declared
  Dept with another wrapper.
- constructor proof that `RdbObjActor<B>` accepts only
  `ObjActorAccess<B::Dept>`, while business composition accepts only
  `ObjAccess<B::Dept>`.

The fixture covers missing, detached, pending, verified, and invalid nullable
tuples. Invalid tuples return an invariant error; they are not normalized by
SQL constraints.

## Stage 3: Implement the typed actor

Implement the private task and `RdbObjActor<B>` behind the RDB ObjDept
feature.

Before inserting its first message:

- add exact-topic polling for each typed actor;
- change the legacy Prom poller to an explicit legacy-topic set;
- test that the legacy poller cannot claim `obj_dept:*`.

Actor deliverables:

- private Check/Delete payload only;
- deterministic unambiguous task id;
- identical conflict means already bound;
- operator-required conflict fails the producer transaction;
- complete RDB classification including invalid tuples;
- physical calls outside transactions;
- affected-row-zero re-read after compare-and-set;
- unlimited transient retries with capped delay;
- worker-timeout recovery without attempt-count death;
- no automatic purge for operator-required object records;
- lease-fenced finalization with affected-row verification.

Focused event cases:

1. current verified Check completes without physical access;
2. current pending plus present object becomes verified;
3. current pending plus absent object retires before cleanup;
4. stale Check and Delete remove only the exact old key;
5. detached same-version cleanup succeeds;
6. current-version Delete refuses physical deletion;
7. task version above watermark refuses physical deletion;
8. invalid tuple refuses physical deletion;
9. overlapping leases cannot delete a row another lease verified;
10. actor death before and after remote deletion is idempotent;
11. actor death after verified compare-and-set completes safely;
12. transient delete failure remains durable beyond three attempts;
13. operator-required record survives normal purge;
14. duplicate task bind with identical payload is idempotent;
15. duplicate task bind with different payload fails.

## Stage 4: Implement pure binding transitions

Add application-level pure logic for:

- current meta read;
- no-slot decision for matching verified content;
- next-version planning for every new slot;
- replacement;
- detach;
- permanent owner deletion;
- overflow;
- invalid tuple rejection.

Every candidate slot flow follows prepare, bind, expose:

1. read snapshot;
2. plan new version and absolute expiry;
3. sign candidate outside transaction;
4. lock and validate snapshot;
5. reject a candidate whose remaining lifetime is already insufficient;
6. persist binding and Check/Delete messages;
7. commit;
8. re-check remaining lifetime;
9. expose only a candidate that passes the post-commit check; otherwise retire
   the committed version and replan with a newer version.

Required race tests:

- concurrent reservations allow only committed candidates to be exposed;
- an unexposed candidate from a failed transaction is discarded;
- same pending plus a new slot increments version;
- a lost response followed by retry increments version;
- Check visibility equals absolute expiry plus the wrapper settle duration;
- a slot with insufficient remaining lifetime is never exposed, including
  when the insufficiency is detected only after commit;
- batch conflict discards and replans the whole candidate set.

The R2 Page path does not claim a finite upload-completion bound. Implement a
typed periodic `ObjReconciler<B>` before Page cutover:

- add a private per-binding inventory capability, not a fifth business
  operation;
- list and parse only the immutable namespace for `B`;
- compare each exact key with typed RDB binding state;
- persist a per-key obligation-generation watermark and scan cursor;
- retain the per-key watermark for the binding namespace lifetime, independent
  of Completed-task and ordinary journal retention;
- lock or compare-and-set that row so concurrent observations allocate at most
  one next generation;
- reuse an unfinished generation, but allocate and bind a new deterministic
  Delete generation when the prior one is Completed and the key is observed
  again;
- treat an absent current task after valid Completed-task purge as settled and
  allocate only `generation + 1`; block on operator-required state;
- keep task status as the sole obligation status and make generation overflow
  operator-required instead of wrapping;
- never physically delete from the reconciler;
- traverse every inventory page, retry a failed cursor, tolerate duplicate
  pages, and require eventual enumeration of every continuously present key;
- preserve current, future, malformed, and invalid-state keys for normal or
  operator handling;
- prove the legacy poller cannot claim reconciliation-produced tasks.

The decisive test first completes an ordinary Delete while the key is absent,
then materializes that old key, runs reconciliation, and proves a new
obligation generation is bound and the actor eventually deletes it. Pagination
failure and restart tests prove the durable cursor cannot skip a page.
Retention and concurrent-scan tests prove task purge cannot reset a generation
or bind the same next generation twice.

## Stage 5: Build Page migration fixtures

Add migration tests that start immediately before the new migration, insert
legacy fixtures, and then apply the new migration.

Fixtures:

- complete verified tuple;
- complete pending tuple;
- all-null tuple;
- partial tuple expected to fail preflight;
- legacy Page Check/Delete rows in every status;
- old key using chapter id and ext.
- absent Page activation marker;
- present Page activation marker after representative bindings and tasks have
  been removed.

Assertions:

- exact values copied for an approved complete tuple;
- all-null handling uses the new non-colliding namespace rule;
- partial tuple aborts without durable schema policy;
- legacy unfinished task gate works;
- Page owner and counters remain unchanged;
- timestamp bootstrap semantics are explicit;
- generated schema has exact standardized types;
- all migration `up.sql` files can be executed twice by the production-style
  runner;
- disposable CI still completes apply, revert-all, apply.
- down succeeds before activation;
- down refuses forever after activation and leaves schema and data unchanged.

Do not use empty-schema CI reversal as evidence that data copy works.

## Stage 6: Migrate physical Page objects

Build a one-time resumable migration tool with a durable journal.

Per verified row it records:

- owner id and version;
- stored old key;
- derived new key;
- copy state;
- verification state;
- old-key cleanup state.

The migration control data also owns a permanent Page activation marker. It is
not subject to normal task, journal, or business-row retention.

The tool copies and verifies before RDB cutover. It does not run through the
generic actor because the old key grammar carries legacy business data.

Cutover gates:

- zero partial legacy tuple;
- zero unresolved legacy pending row, unless the approved retirement rule was
  selected;
- zero unfinished legacy Page object message;
- every verified old object copied and verified;
- old keys retained through the rollback window.

The tool is idempotent, restartable, and reports exact remaining counts.

## Stage 7: Expand and cut Page over

Use separate expand and contract migrations.

Expand:

- create `t_page_image` with only the exact standardized shape;
- keep the old Page columns;
- make the SQL repeatable;
- do not add foreign keys, tuple checks, or version-order checks.

Maintenance cutover:

1. make ingress reject all Page object mutations and producers;
2. wait for already admitted producers to finish;
3. drain remaining legacy Page work;
4. stop the legacy actor and old application;
5. with no writer, re-run task, pending, partial, and copy gates;
6. perform the final idempotent RDB copy;
7. start the new application in cutover-validation mode while ingress remains
   frozen and every Page actor, reconciler, scheduler, and producer is off;
8. run read-only health and focused RDB/remote consistency checks;
9. write the permanent Page activation marker;
10. start background components and producers, then unfreeze ingress; test
    rollback before the marker and forward-only repair after it.

The validation-mode test runs longer than one reconciliation tick and proves
that no typed task or binding write occurs. A marker-before-startup test proves
that background startup failure cannot make generic down legal again.

Application cutover:

- Page manifest and single reserve use the same pure planner;
- candidate slots are prepared outside and exposed after the owner transaction;
- Page publish detaches and retains watermark;
- Page deletion binds cleanup before removing binding and Page rows;
- reads generate URLs only for verified bindings;
- Page workflow progression stays outside the object actor;
- mock and RDB bindings share the same state-transition cases;
- no dual writer remains after the maintenance transaction.

The current production script runs migrations before stopping the old
container. It cannot execute this contract safely. Production cutover waits
for a reviewed GitHub Actions maintenance choreography or a separate bounded
compatibility design. No local deployment helper is used.

## Stage 8: Static runtime composition

The application composition root explicitly owns and closes one
`RdbObjActor<Page>` and one `ObjReconciler<Page>` beside the scheduler.

Harn receives only ObjDept capabilities used by use cases. It does not gain one
generic parameter for every actor binding.

Restart test:

1. leave Delete unfinished;
2. stop the actor after lease acquisition;
3. start a new actor instance;
4. prove exact-key deletion completes;
5. prove the durable record completes under the new lease.

## Stage 9: Contract cleanup

After the rollback window and all gates:

- remove old Page object columns in a separate repeatable migration;
- remove legacy Page object handler branches;
- retain old-key journal cleanup until every old key is gone;
- remove Page from legacy topic ownership;
- update `docs/image-consistency.md` to the checked-in path;
- delete completed migration notes instead of preserving a stale plan.

Down SQL checks the permanent Page activation marker before destructive work.
Rollback after activation is an explicit operational data migration.

## Stage 10: Add wrappers one at a time

For user avatar, team avatar, comic cover, font, and software:

1. define wrapper policy;
2. declare the static binding table;
3. run key/data/task migration gates;
4. add the exact-topic actor to the composition root;
5. cut the wrapper over;
6. contract the legacy path;
7. review before starting the next wrapper.

Adding a wrapper must not add a central payload match or runtime handler map.

## Validation order per implementation slice

```text
cargo fmt --all --check
cargo check --all-features
cargo test -p poprako-obj-dept
cargo test -p poprako-rdb-impl --all-features
cargo test -p poprako-server
```

Migration slices also run the checked-in disposable CI migration validation
and the dedicated data fixture tests. No local release, image build, or
deployment helper is run.

## Stop conditions

Stop and redesign if implementation requires:

- a business kind inside the private ObjTask;
- a runtime map from kind to table or handler;
- a hidden macro-generated binding selected by payload;
- `poprako-rdb-impl` depending on `poprako-server`;
- a physical request while an owner transaction is open;
- exposing a slot before its binding and Check commit;
- reusing a version for another exposed slot;
- completing Check before the proven write quiet point;
- treating compare-and-set affected-row zero as stale without re-read;
- committing Delete separately from its owner-state transition;
- letting retry count or purge erase an unresolved delete obligation;
- storing the physical key in the standardized binding table;
- encoding mutable wrapper policy as a durable database constraint;
- cutting Page over before old keys, tasks, and pending rows are resolved;
- relying on the current migrate-before-stop production order for a breaking
  contract migration.
- handing a use case the complete adapter or an actor-side physical operation;
- allowing actor composition to override the Dept fixed by its binding macro;
- claiming Page R2 cleanup reliability without typed remote reconciliation;
- permitting generic down after the permanent Page activation marker exists.
