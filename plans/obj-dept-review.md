# ObjDept Review Resolution

Status: v3 review passed

Reviewed files:

- `specs/obj-dept.md`
- `plans/obj-dept.md`

Independent review tracks:

- static Rust dispatch and macro coherence;
- operation and crate boundary;
- lifecycle and deletion races;
- RDB split and migration safety.

## Resolved blockers

| Review finding | Resolution in v2 |
| --- | --- |
| binding was absent from operation types | every operation is typed by `B`; tasks still serialize only id and version |
| natural macro expansion could violate coherence | annotated local type is the binding; generic operation impls use local `RdbObjRepo<B>` self type |
| macro belonged to the RDB implementation boundary | renamed to `poprako-rdb-impl-macro` |
| RDB crate could form a dependency cycle | fixed one-way dependency graph and neutral `RdbError` boundary |
| old Prom actor would claim new topics | exact typed topic polling plus explicit legacy topic ownership is a precondition |
| same pending version could be re-signed | every exposed slot receives a new version; tasks are never re-armed |
| Check could run before the slot became quiet | absolute expiry plus a proven settle duration |
| compare-and-set zero could delete a verified object | mandatory re-read and same-version verified completion |
| retry or purge could erase cleanup debt | unlimited transient retry and no automatic purge of operator-required object records |
| new key could not locate current Page objects | explicit resumable old-key to new-key migration journal |
| backfill and drop was incompatible with deployment order | separate expand/cutover/contract stages and maintenance choreography gate |
| empty CI reversal did not test data migration | pre-migration fixtures and production-style repeat execution |
| partial tuples could be silently lost | migration preflight refusal without a permanent tuple constraint |
| macro wrapper token did not constrain actor composition | `RdbObjBinding::Dept` fixes the wrapper and `RdbObjActor<B>` uses only `B::Dept` |
| prose alone did not prevent business physical deletion | opaque business and actor capability wrappers make the split compile-time |
| a prepared slot could expire while waiting for the transaction | pre-write and post-commit lifetime checks; a late failure is never exposed |
| a one-shot Check could miss an arbitrarily late R2 upload | typed periodic inventory binds Delete; reconciliation never deletes directly |
| task retention could erase evidence needed by down refusal | permanent Page activation marker independent of task and journal retention |
| the maintenance freeze left an in-flight writer race | explicit ingress freeze, producer drain, actor/app stop, final gates, frozen startup, marker, unfreeze |
| associated Dept was confused with the runtime facade | actor constructor accepts `ObjActorAccess<B::Dept>`; business receives `ObjAccess<B::Dept>` |
| a late stale key could collide with a Completed Delete | reconciliation persists a new obligation generation and never re-arms the old task |
| inventory paging could silently miss remote keys | durable cursor, failed-page retry, idempotent duplicate pages, and eventual-enumeration contract |
| new background workers could write before activation | cutover-validation mode starts no Page writer; workers start only after the permanent marker |
| task retention could reset reconciliation generation | permanent per-key watermark, task-status truth, and locked/CAS next-generation allocation |

## Deliberate decisions

- The actor Check verifies physical existence only. It does not interpret
  `hash` or `ext`.
- `hash` and `ext` remain business metadata used to plan versions and map DTOs.
- A detached watermark is retained because otherwise an old Delete can race a
  reused key.
- The public `DelObjs<B>` operation is a physical primitive owned at runtime by
  the actor facade. The business facade cannot satisfy that operation bound;
  business deletion only binds a durable Delete task inside its RDB
  transaction.
- The current Page key grammar is treated as legacy data, not preserved in the
  standardized binding table.
- The R2 Page pilot requires periodic reconciliation because the reviewed
  backend contract does not establish a finite completion bound for an upload
  already in progress. A one-shot Check cannot prove eventual cleanup alone.
- Reconciliation observations use new durable obligation generations so a
  previously Completed Delete cannot swallow a later cleanup obligation.

## Remaining implementation proof

No design-review blocker remains. Page migration still must not begin until
Stage 0 through Stage 3 prove:

1. the exact generic operation signatures compile;
2. macro expansion satisfies coherence for two fake bindings;
3. the RDB error/context extraction has no dependency cycle;
4. topic ownership prevents cross-claiming;
5. actor overlap and retry tests match the state machine;
6. R2 reconciliation rediscovers and retires a late stale object;
7. the Page activation marker permanently fences unsafe down migration.

Any failure in those proofs returns the design to spec review before schema or
Page code changes.
