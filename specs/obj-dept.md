# ObjDept Reliable Object Lifecycle Spec

Status: reviewed draft v3

This revision incorporates independent reviews of static dispatch, operation
boundaries, lifecycle races, and RDB migration safety.

## 1. Objective

ObjDept manages remote objects without understanding their business payload.
Image, font, and software are typed wrappers over the same object lifecycle.

The contract is:

- one business owner has one stable `id`;
- `ObjKey { id, version }` is the complete ObjDept identity;
- ObjDept assembles the physical storage key;
- the storage backend receives only that assembled key;
- RDB keeps only the latest binding and its version watermark;
- delayed checks and failed deletes allow zero to many physical objects to
  exist for one owner at one time;
- every cleanup obligation is persisted in the same RDB transaction as the
  state change that creates it;
- the RDB actor understands only Check and Delete;
- table and handler selection are compile-time, not payload-driven.

## 2. Public part boundary

`poprako-obj-dept` owns the existing public names:

- `ObjKey` and `ObjKeyRef`;
- `ObjSpec`, `ObjMeta`, and `ObjSlot`;
- `GetObjMeta`;
- `GenObjUrl`;
- `GenObjSlot`;
- `DelObjs`;
- the typed operation capabilities described below.

`B` is a zero-sized binding marker. It is carried by the operation type through
`PhantomData`, not serialized into a task. This lets one adapter implement
multiple statically selected object bindings without a runtime kind.

The four operations are storage-part operations:

| Operation | Storage meaning | Caller |
| --- | --- | --- |
| `GetObjMeta<B>` | Read whether the assembled key currently exists and return storage-level metadata. | RDB actor Check |
| `GenObjUrl<B>` | Generate a short-lived read capability for the assembled key. | typed business wrapper |
| `GenObjSlot<B>` | Generate a write capability for the assembled key with an explicit absolute expiry. | typed business wrapper |
| `DelObjs<B>` | Idempotently remove the exact assembled keys; absence is success. | RDB actor Delete or stale Check |

`DelObjs<B>` is the physical primitive, not the business deletion API. This is
enforced by types, not only by composition convention:

- `ObjAccessDept<B>` contains only `GenObjUrl<B>` and `GenObjSlot<B>`;
- `ObjActorDept<B>` contains only `GetObjMeta<B>` and `DelObjs<B>`;
- the complete `ObjDept<B>` adapter may implement both, but remains private to
  the composition root;
- an opaque `ObjAccess<D>` wrapper implements only the business-side operation
  bounds supported by `D` and has no accessor to the complete adapter;
- an opaque `ObjActorAccess<D>` wrapper implements only the actor-side
  operation bounds supported by `D`.

Harn and use cases receive `ObjAccess<D>`. The typed actor receives
`ObjActorAccess<D>`. Business deletion calls its typed RDB binding inside the
owner transaction and cannot call the physical delete operation.

`ObjDept<B>` does not depend on Page, Image, Font, Software, Diesel, local
messages, or the application error type. Storage adapters expose a small error
classification that tells the actor whether an operation is retryable. The
adapter logs its SDK error once before converting it.

`ObjSpec` keeps `ObjKeyRef` as its core. A wrapper can add non-persisted write
requirements such as byte length and media type without making those fields
actor semantics. `ObjSlot` carries the URL, all required signed headers, and
the exact `write_expires_at` used to create the capability.

The actor does not compare or interpret `hash` or `ext`. Those values belong to
business identity and version planning.

## 3. Crate and dependency boundary

The dependency direction is fixed:

```text
poprako-obj-dept
        ↑
poprako-rdb-impl
        ↑
poprako-server

poprako-rdb-impl-macro ── expansion references ──> poprako-rdb-impl
poprako-server depends on the macro crate directly
```

`poprako-rdb-impl-macro` is a small proc-macro crate because the attribute
binds a Diesel table. It is not part of the storage-neutral ObjDept crate.

`poprako-rdb-impl` owns:

- `RdbCore` and `RdbContext`;
- a generic `RdbNucl<E>` or equivalent transaction coordinator that does not
  name the application error type;
- `RdbError`;
- storage-neutral local-message row, lease, claim, retry, and completion
  mechanics;
- the private serialized `ObjTask::{Check, Delete}`;
- `RdbObjBinding`;
- `RdbObjRepo<B>`;
- `RdbObjActor<B>`.

The application implements conversion from `RdbError` to its application
error at the adapter boundary. `poprako-rdb-impl` never depends on
`poprako-server`.

Migrations and application table declarations remain owned by
`poprako-server`.

## 4. Static RDB binding

The adapter-local declaration is:

```rust
#[obj_dept(Image, t_page_image)]
pub struct Page;
```

The exact expansion rules are:

- the annotated local type `Page` is the binding marker;
- `Image` selects the typed ObjDept wrapper;
- `t_page_image` selects the Diesel table at compile time;
- the macro implements `RdbObjBinding` for the local marker;
- it emits concrete, mechanical Diesel access methods for the standardized
  columns;
- it derives the unique topic `obj_dept:t_page_image` from the table token;
- it does not generate a hidden marker through identifier concatenation;
- it does not generate an implementation with both a foreign trait and a
  foreign self type.

The expansion also binds the two type dimensions explicitly:

```rust
impl RdbObjBinding for Page {
    type Dept = Image;
}
```

`RdbObjActor<B>` stores an `ObjActorAccess<B::Dept>` value, with
`ObjActorAccess<B::Dept>: ObjActorDept<B>`. Its constructor requires exactly
that facade. The associated type fixes the adapter family; it does not stand
in for the runtime adapter value. There is no separately selectable actor type
parameter that could pair `Page` with a wrapper other than `Image`.

The business side correspondingly requires
`ObjAccess<B::Dept>: ObjAccessDept<B>`. The complete adapter exists only long
enough in the composition root to construct the two opaque facades.

`RdbObjRepo<B>` is owned by `poprako-rdb-impl`, so generic Orchestra operation
implementations use a local self type and satisfy coherence rules.

The generated methods contain table-specific query expansion because fully
abstract Diesel table and column associated types are not a realistic trait
surface. The macro does not contain permission, version-allocation, hash, ext,
or owner policy.

## 5. RDB representation

The Page pilot splits `t_page` into owner data and `t_page_image` binding data.

`t_page` retains page id, chapter id, index, counters, and timestamps. It has
no object-storage state after contract cleanup.

The exact `t_page_image` shape is:

```text
f_id            TEXT        PRIMARY KEY
f_version       BIGINT      NOT NULL
f_is_uploaded   BOOLEAN
f_hash          BYTEA
f_ext           TEXT
f_created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
f_updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
```

No owner foreign key, tuple `CHECK`, version-order `CHECK`, historical-version
row, or stored physical key is added. Changeable tuple rules stay in Rust.

The business layer decodes the nullable row into:

| RDB form | Rust meaning |
| --- | --- |
| no row | this key namespace has never allocated a version, or the owner was permanently removed |
| `f_is_uploaded`, `f_hash`, and `f_ext` all null | detached watermark |
| `f_is_uploaded = false`, with `f_hash` and `f_ext` | current pending binding |
| `f_is_uploaded = true`, with `f_hash` and `f_ext` | current verified binding |
| any other tuple | invariant failure; never guess and never delete from it |

Detach retains `f_version`, clears the active tuple, and binds Delete for the
last version. The next slot uses the next version. This prevents an old Delete
attempt from racing a newly reused key.

Owner ids are never reused. Versions only increase for one owner. Overflow is
an application error. These are business invariants with focused Rust tests,
not SQL policy.

## 6. Physical key contract

The new key grammar is a pure, stable serialization of
`ObjKey { id, version }` inside the typed ObjDept adapter. A constant grammar
namespace may prefix the serialization, but `hash`, `ext`, chapter id,
timestamps, and RDB contents never affect it.

The namespace is immutable while physical objects or durable tasks using it
exist. A grammar change is a remote-data migration, not a refactor.

The current Page key uses chapter id and ext. Therefore current remote objects
cannot be addressed by the new grammar and require the explicit legacy
migration in Section 13.

## 7. Private durable task contract

`ObjTask` belongs to the RDB implementation feature, not the public part:

```rust
enum ObjTask {
    Check { key: ObjKey },
    Delete { key: ObjKey },
}
```

One message owns one key. `DelObjs` may accept a slice, but v1 calls it with a
one-key slice so failures are isolated.

Task ids are an unambiguous stable encoding of topic, operation, owner id,
version, and obligation generation. Ordinary owner transitions use their fixed
generation. A reconciliation observation allocates its generation durably as
described in Section 11.1. Tasks are not random and are never re-armed.

Binding an already existing deterministic id has these semantics:

- Pending, Processing, or Completed with the identical topic and payload is
  already bound and succeeds without mutation;
- a different topic or payload is an invariant error;
- an operator-required record is an invariant error, so a business
  transaction cannot pretend the cleanup was rebound.

Messages may be delivered more than once and out of order. No transition
relies on message order.

## 8. Slot planning and reservation

`version` identifies one write-capability generation. Every slot that can be
exposed to a caller uses a newly allocated version, even when `hash` and `ext`
match a pending row.

The safe order is prepare, bind, expose:

1. Read the owner and binding snapshot.
2. Pure business logic decides whether no slot is needed or plans the next
   version, `hash`, `ext`, and an absolute `write_expires_at`.
3. Call `GenObjSlot<B>` outside an RDB transaction for the candidate key and
   the exact absolute expiry.
4. Open the owner transaction and lock in the canonical owner/binding order.
5. Re-read and compare the snapshot. On conflict, roll back, discard the
   unexposed slot, and retry with a new plan.
6. Re-check that enough lifetime remains to commit and expose. If not, roll
   back and replan without writing the candidate.
7. Persist the new pending binding.
8. Bind Delete for the previous active version, when one exists.
9. Bind Check for the new version with
   `visible_at = write_expires_at + settle_duration`.
10. Commit the owner change, binding change, and messages.
11. Re-check the remaining lifetime before exposing the slot. If it is now too
    short, never expose it; treat it as a lost response, allocate a newer
    version, and retire this committed version through the normal path.
12. Only a candidate that passes the post-commit check may be returned.

If signing fails, no RDB write has occurred. If the transaction fails, the
signed capability was never exposed and is discarded. If the response is
lost, a retry allocates a new version and retires the old one; it does not
re-arm Check for the old version.

Batch Page reservation prepares candidate slots concurrently, then validates
and commits all Page and binding changes in one transaction. A conflict
discards every unexposed candidate and replans the batch.

The storage wrapper must guarantee a finite maximum duration for an upload
that begins before `write_expires_at`. `settle_duration` covers that bound plus
storage visibility delay. The reviewed R2 contract documents signed-request
expiry but does not establish this upload-completion bound. Therefore the Page
R2 pilot requires the periodic reconciliation mechanism in Section 11.1; a
one-shot Check is not claimed as the complete reliability mechanism.

`GenObjSlot<B>` rejects an absolute expiry with too little remaining time. A
rejected candidate is never exposed and the application replans.

## 9. Check actor

The actor handles only the existence meaning of Check. `hash` and `ext` remain
opaque row fields used for a complete compare-and-set; the actor does not
interpret them.

The first RDB classification is:

- same version and verified: complete without a remote call;
- same version and pending: call `GetObjMeta<B>` outside the transaction;
- detached with task version at or below the watermark: cleanup-only;
- active with a greater current version: cleanup-only;
- missing row: cleanup-only;
- task version above the watermark: operator-required, no delete;
- invalid nullable tuple: operator-required, no delete.

For a current pending key:

- present: compare-and-set that exact pending row to verified;
- absent after the write quiet point: compare-and-set that exact pending row to
  detached, then run one idempotent delete for the retired key;
- storage error: retry without changing the binding.

The compare-and-set includes id, version, pending state, and the opaque active
tuple read before the remote call.

Affected-row zero is never treated as stale by assumption. The actor re-reads:

- same version verified: another lease completed the work; complete and do not
  delete;
- same version pending: retry classification and do not delete;
- detached, missing, or greater version: cleanup-only;
- higher task version or invalid tuple: operator-required and do not delete.

Cleanup-only calls `DelObjs<B>` for the exact old key and completes only after
idempotent deletion succeeds.

Check never advances Page workflow. Application workflow code observes
verified binding state separately.

## 10. Delete actor

Delete classifies the exact key:

- missing row: safe to delete;
- detached and task version at or below the watermark: safe to delete;
- active and task version below current: safe to delete;
- active and the same version: operator-required, do not delete;
- task version above the watermark: operator-required, do not delete;
- invalid tuple: operator-required, do not delete.

After safe classification, `DelObjs<B>` runs outside the RDB transaction.
Absence is success.

The classification-to-delete race is safe only because owner ids are not
reused and versions never decrease. A newer active key must have a greater
version.

## 11. Retry, lease, and retention

ObjDept tasks have operation-specific policy:

- transient storage and RDB failures retry without an attempt limit;
- delay uses capped backoff, but obligation lifetime is not capped;
- a worker timeout returns the record to Pending and does not convert it into
  an operator-required state merely because of lease count;
- malformed payload, impossible version direction, invalid tuple, and current
  Delete become operator-required;
- operator-required object records are never automatically purged;
- Completed records may be purged after the normal retention window;
- completion and retry match `(message id, lease)` and verify the affected row
  count.

An operator repair is explicit and audited. A later business transaction may
not overwrite the evidence silently.

### 11.1 Remote reconciliation

The R2 Page binding runs a typed periodic `ObjReconciler<B>` in addition to
`RdbObjActor<B>`. It closes the case where an upload begins before slot expiry
but becomes visible only after the one-shot Check has retired that version.

Reconciliation uses a private maintenance inventory capability, separate from
the four public operations. It lists keys only under the immutable namespace
for `B`, parses each key as an `ObjKey`, and compares it with the typed binding
state. It does not inspect `hash`, `ext`, or payload bytes.

For an exact remote key that is missing, detached, or older than the active
watermark, the reconciler creates or observes a durable per-key reconciliation
watermark in an RDB transaction. This generation watermark is not task data:
it is retained for the lifetime of the binding namespace, never decreases, and
is not removed by Completed-task or ordinary journal retention.

Concurrent observations lock or compare-and-set the same per-key watermark
row. The task named by its current generation is the obligation-state truth:

- Pending or Processing means reuse the current generation;
- Completed, or absence after normal Completed-task purge, permits allocating
  `generation + 1` when the remote key is observed again;
- operator-required blocks allocation and preserves its evidence.

There is no duplicated obligation status for the actor to synchronize. Task
status is the only status, and the watermark row is only monotonic identity.
Generation overflow becomes operator-required and never wraps.

Watermark allocation and new deterministic Delete binding commit in the same
transaction. Therefore a generation is never reused, including after task
purge. The reconciler never re-arms an old task and never calls physical
deletion directly.

The actor payload remains exactly `Delete { key }`; obligation generation is
message identity and reconciliation bookkeeping, not payload meaning. A
current active key is retained. A key above the watermark, an invalid key, or
an invalid row becomes an audited operator issue and is not guessed safe.

An inventory run owns a durable scan cursor. A run completes only after every
page reaches the terminal cursor. A failed page retries from its last durable
cursor, and duplicate page delivery is idempotent. The inventory adapter must
guarantee that a continuously present key is eventually returned by a
completed scan. Without complete pagination, retry, and this eventual
enumeration property, the wrapper cannot claim reliable reconciliation.

The reconciler is monomorphized by `B`, inventories only that binding's
namespace, and does not introduce a runtime business-kind registry. The actor
still receives and handles only Check and Delete.

## 12. Detach and owner deletion

Detach is one owner transaction:

1. lock owner and binding;
2. bind Delete for the active key, if one exists;
3. retain `f_version`;
4. clear `f_is_uploaded`, `f_hash`, and `f_ext`;
5. apply the owner change;
6. commit.

Permanent owner deletion is also one transaction:

1. lock owners and bindings in canonical order;
2. bind Delete for every active key;
3. remove binding rows;
4. remove owners and related aggregates;
5. commit.

The absence of a foreign key is deliberate. Transaction tests, not a mutable
database policy, prove both sides change together or neither changes.

## 13. Page data and physical-object migration

The legacy Page key is not compatible with the new key grammar. The migration
therefore has three separate obligations.

### 13.1 Legacy task gate

Before Page cutover, inventory legacy Page Check/Delete messages in every
status. Either drain them while the old columns and actor still exist, or
translate them through an explicitly tested one-time path. The contract gate
requires zero unfinished legacy Page object messages.

The local database currently has one Completed legacy image message and no
unfinished legacy Page object message. This local fact is not a production
assumption.

### 13.2 Remote key copy

Verified legacy objects are copied from their stored old key to the new key
derived from the same id and version. A one-time durable migration journal
records old key, new `ObjKey`, copy state, verification state, and later old-key
cleanup state. A separate durable Page activation marker is written at
successful cutover and is never removed by task or journal retention.

This journal worker is a migration tool, not the generic Check/Delete actor. It
is idempotent and resumable. Old keys remain until the rollback window closes.

Legacy pending rows are not silently converted. Cutover requires them to be
resolved, or an approved migration rule retires them and requires a new slot.
Partial legacy tuples fail preflight rather than being dropped.

All-null legacy tuples have already lost their old watermark. They allocate
from version 1 in the new, non-colliding key namespace. Outstanding old-key
deletes remain owned by the drained legacy task path.

### 13.3 RDB split

The expand migration creates `t_page_image` without dropping old Page columns.
It is safe to execute repeatedly.

The exact maintenance choreography is:

1. ingress rejects every Page object mutation and every producer that can bind
   Page object work;
2. wait for already admitted producers to finish;
3. allow the legacy actor to drain its remaining Page work;
4. stop the legacy actor and old application, leaving no Page object writer;
5. re-run pending-row, partial-row, legacy-task, and remote-copy gates;
6. perform the final idempotent RDB copy;
7. start the new application in cutover-validation mode while ingress remains
   frozen; this mode does not start the Page actor, reconciler, scheduler, or
   any component that can write Page bindings or typed tasks;
8. run read-only health and focused RDB/remote consistency checks;
9. write the permanent Page activation marker;
10. only after the marker commits, start the Page actor, reconciler, scheduler,
    and other producers, then unfreeze ingress. Before the marker, failure may
    reverse the expand schema and restore the old application; after the
    marker, recovery is forward-only.

The later contract migration removes old columns only after the new path is
stable and rollback no longer depends on them. Every `up.sql` remains safe when
the production migration runner executes the full set again.

The current deployment script migrates before stopping the old container.
Therefore a one-release backfill-and-drop is forbidden. Production cutover
requires an approved maintenance choreography in GitHub Actions or a separately
reviewed bounded compatibility rollout.

Down migration is supported only while the permanent Page activation marker
is absent. Once present, generic down refuses forever even if bindings, tasks,
or migration-journal rows have later been removed. The refusal is checked
before destructive SQL, so a failed down leaves schema and data unchanged.
After activation, rollback is an operational data migration, not a generic
`down.sql` promise.

## 14. Topic ownership and runtime composition

Each macro binding derives one unique table-based topic. Its actor polls only
that exact topic.

Before the first ObjDept task is inserted:

- typed Obj actors use exact-topic polling;
- the legacy Prom actor changes from all-topic polling to an explicit set of
  legacy topics;
- it cannot claim an ObjDept record and decode it as the application payload.

Topic filtering in SQL is not handler dispatch. The handler remains
monomorphized as `RdbObjActor<Page>`, and `Page::Dept` fixes the Image adapter.

Background actors are owned and closed by the application composition root
beside the existing scheduler. They are not added as one generic parameter per
binding to Harn. Harn carries only capabilities directly used by use cases.

## 15. Failure closure

| Failure | Durable result | Recovery |
| --- | --- | --- |
| slot signing fails | no RDB change | caller retries |
| bind transaction fails | unexposed candidate slot only | discard and replan |
| commit succeeds and response is lost | pending version plus Check | retry allocates a newer version and binds old Delete |
| commit succeeds but the slot is too near expiry | committed pending version is never exposed | allocate a newer version and retire the unexposed one |
| client never uploads | pending until quiet-point Check | Check retires to detached and performs idempotent cleanup |
| replacement wins before old Check | newer pending binding plus old Delete | old Check is cleanup-only |
| actor dies after retire and before delete | detached watermark plus unfinished Check | reclaimed Check continues cleanup |
| actor dies after remote delete | unfinished task | idempotent delete repeats |
| actor dies after verified CAS | verified row plus unfinished Check | reclaimed Check completes without remote delete |
| two leases overlap | affected-row zero forces re-read | same-version verified is never deleted |
| remote delete fails | unfinished durable obligation | unlimited retry with capped delay |
| owner deletion commits before Check | missing binding plus bound Delete | Check or Delete removes exact old key |
| invalid nullable tuple | unchanged row plus operator-required task | explicit repair; no guessed deletion |
| a very late upload appears after Check | stale remote key found by typed reconciliation | bind durable Delete; actor performs idempotent cleanup |

## 16. Acceptance criteria

- existing `oper` names remain the ObjDept part boundary;
- every operation is statically parameterized by binding type;
- business code cannot obtain `GetObjMeta` or `DelObjs` capability;
- the macro fixes `B::Dept`; actor composition cannot select a mismatched
  wrapper;
- actor payload is private to RDB impl and has only Check/Delete;
- actor interprets existence and version state, not business payload,
  `hash`, or `ext`;
- every exposed slot has a newly allocated version and a durable Check after
  its proven write quiet point;
- every detach, replacement, and owner deletion binds Delete in the owner
  transaction;
- no physical request runs while an owner transaction is open;
- no physical key is stored in the standardized table;
- current Page objects are copied to the new key before cutover;
- legacy tasks and pending rows have explicit gates;
- static topic ownership prevents the legacy actor from claiming ObjDept work;
- transient deletion failures never lose their obligation to retry count or
  automatic purge;
- Page R2 remote reconciliation can rediscover a late stale object and bind a
  new-generation durable Delete without deleting directly or reusing an old
  message id;
- the permanent activation marker makes unsafe generic down impossible even
  after normal data retention;
- another wrapper requires a binding declaration and business policy, not a
  new central runtime match.
