# ObjDept Latest TO-FIX Correction Plan

## 1. Scope

This plan replaces the previous ObjDept plan. It covers every unchecked item in
`docs/to-fix.md` and the later decisions that:

- `_m` remains the only PhantomData field name;
- `obj_inst!` hides `_m` from use-case construction;
- ObjDept owns exactly one `ObjActor` and one `ObjActorDesc`;
- object topics are typed handler branches inside that actor;
- the first actor implementation processes every topic serially;
- future topic parallelism remains private to `ObjActor`;
- chapter-stage advancement remains a business Prom responsibility.

This document is a plan only. Rust, migrations, and generated schema must not
change until the plan and updated spec are accepted.

The dependency direction remains:

```text
poprako-rdb-core <- poprako-obj-dept <- poprako-server
```

No rejected contract may survive through an alias or compatibility wrapper.

## 2. Ownership

`poprako-rdb-core` owns only neutral connection and transaction mechanics.

`poprako-obj-dept` owns:

- `ObjDept`, `ObjPool`, `ObjProm`, and `ObjPromDefer<C>`;
- object keys, metadata, upload capabilities, and durable task values;
- ObjDept opers, `ObjDeptError`, and `ObjDeptRest`;
- one `ObjActor` event loop and its `ObjActorDesc`;
- `obj_inst!`, `objs_def!`, `rdb_obj_prom!`, and `impl_obj_dept!`;
- RDB-feature glue that never erases a concrete Diesel table.

`ObjPromDefer<C>` is not a renamed Bind abstraction. It contains only the
explicit caller-transaction actions `defer_check` and `defer_delete`. It is an
independent trait, has no `ObjProm` supertrait, and is never required at the
actor boundary.

`poprako-server` owns:

- generated Diesel schema and migrations;
- the R2 struct that directly implements `ObjPool`;
- the RDB ObjProm declaration against `t_obj_prom_task`;
- PageImage, UserAvatar, TeamAvatar, and ComicCover markers;
- one `objs_def!` invocation;
- one total `NormObjDept` composition;
- use-case, Harn, HTTP, and business Prom integration.

There is no ObjPool-like server port under `part`. R2 is only a production
adapter under `part_impl`.

## 3. Module layout

Remove the meaningless `model::obj` bucket and split values by function:

```text
poprako-obj-dept/src/
├── actor.rs
├── key.rs
├── lib.rs
├── model.rs
├── model/meta.rs
├── model/slot.rs
├── model/task.rs
├── oper.rs
├── pool.rs
├── prom.rs
├── rdb_impl.rs
└── rest.rs
```

- `model::meta`: newest logical object metadata;
- `model::slot`: ObjDept upload capability and meaningful slot inputs;
- `model::task`: Check/Delete task, operation, and actor flow;
- `pool`: physical storage contract;
- `prom`: task producer/consumer contracts;
- `actor`: one event loop and routing;
- `rdb_impl`: typed-row conversion and RDB error mapping only.

`rdb_impl` must not define a load/write/delete trait. Concrete table queries are
emitted at the server macro invocation.

## 4. Naming matrix

| Remove | Replacement or action |
|---|---|
| `RdbObjDept` | `NormObjDept` |
| repeated `obj!` | one `objs_def!` |
| `ObjPoolSpec` | remove; pass `key`, `content_type`, and `byte_len` to `gen_slot` |
| `ObjPromBind` | `ObjPromDefer<C>` |
| `bind_check` / `bind_delete` | `defer_check` / `defer_delete` |
| `BIND` / `f_bind` | `TOPIC` / `f_topic` |
| `BindRow` | remove or use an action-specific row name |
| `ObjActorSettle` / `ObjPromSettle` | remove |
| every `settle` identifier | explicit complete/retry/operator action |
| `start_obj_actors` / `ObjActorDesc::start` | remove |
| `actor_descs` | singular `actor_desc` |
| `ObjActorCtrl` | remove; descriptor directly owns control state |
| oper constructors | `obj_inst!` |
| server `ObjRead` wrapper | direct ObjDept oper use |

`ObjDeptError`, `ObjDeptRest`, `gen_slot`, `gen_url`, `has`, and `del` remain.
Every PhantomData field is `_m`.

Remove meaningless `#[must_use]` attributes throughout the touched ObjDept,
macro output, R2, Harn, and oper paths. An attribute may remain only with a
documented correctness reason.

## 5. `obj_inst!`

ObjDept opers have no inherent constructors. Use cases write only meaningful
fields:

```rust
let oper = obj_inst! {
    GenObjSlot<PageImage> {
        spec: &obj_spec,
    }
};
```

Enum variants use the same form:

```rust
let oper = obj_inst! {
    DelObjs<PageImage>::Remove {
        ids: &page_ids,
    }
};
```

The macro appends `_m: PhantomData` and expands to a literal, never a hidden
constructor.

Requirements:

1. Struct `_m` fields are public and doc-hidden because macro hygiene does not
   bypass field privacy. This is enforced as a source convention and audit,
   not falsely presented as a Rust privacy guarantee. Keeping `_m` private
   would require the public hidden constructor that this design rejects.
2. The expansion uses fully qualified
   `::core::marker::PhantomData`, independent of caller imports.
3. Only explicitly supported ObjDept oper forms are accepted.
4. Missing meaningful fields, unknown fields, unknown variants, and unsupported
   opers fail compilation.
5. Server use cases and tests contain no direct `_m` initialization.
6. Compile-pass and compile-fail fixtures cover every supported shape.

## 6. `objs_def!`

The server imports markers and generated table modules, then invokes one macro
using bare identifiers:

```rust
objs_def! {
    PageImage {
        table: t_page_image,
        topic: "page_image",
        namespace: "page_image",
    },
    UserAvatar {
        table: t_user_avatar,
        topic: "user_avatar",
        namespace: "user_avatar",
    },
    TeamAvatar {
        table: t_team_avatar,
        topic: "team_avatar",
        namespace: "team_avatar",
    },
    ComicCover {
        table: t_comic_cover,
        topic: "comic_cover",
        namespace: "comic_cover",
    },
}
```

No `crate::...` path appears outside `use` declarations in the touched source.

The macro rejects duplicate marker, table, topic, or namespace entries before
emitting code. Duplicate table, topic, and namespace each have a compile-fail
fixture; duplicate marker is rejected by the same parser validation.

The macro emits:

- one private typed Diesel helper per object;
- direct references to all required generated table columns;
- static topic and physical namespace values;
- a hidden local callback manifest consumed by `impl_obj_dept!`;
- all information needed to emit one complete static dispatch function.

The callback ABI is frozen as local item generation, not token fragments:

```rust
macro_rules! __objs_manifest {
    ($callback:ident) => {
        $callback! {
            (PageImage, __obj_page_image, "page_image", "page_image"),
            (UserAvatar, __obj_user_avatar, "user_avatar", "user_avatar"),
            (TeamAvatar, __obj_team_avatar, "team_avatar", "team_avatar"),
            (ComicCover, __obj_comic_cover, "comic_cover", "comic_cover"),
        }
    };
}
```

`impl_obj_dept!` emits a callback `macro_rules!` and invokes
`__objs_manifest!(callback)`. The callback expands complete items, including
the entire `async fn dispatch`; a manifest invocation never tries to expand to
loose match arms. Every match branch calls and awaits its concrete typed helper
inside that branch, so different handler future types are never selected into
one value. Proc-macro global state, boxed futures, and dyn dispatch are
forbidden.

Adding Font requires only its marker, migration/schema, and one manifest entry.
It does not add an actor, Harn field, main dependency, pool/prom trait, or manual
ObjDept implementation.

## 7. Diesel type safety

Each generated object helper must satisfy a real `Selectable` or explicitly
typed Diesel tuple contract with exactly these generated columns and types:

```text
f_id          Text
f_version     BigInt
f_is_uploaded Nullable<Bool>
f_hash        Nullable<Bytea>
f_ext         Nullable<Text>
f_created_at  Timestamptz
f_updated_at  Timestamptz
```

`rdb_obj_prom!` must satisfy the same kind of real typed contract for the full
task row:

```text
f_id              Text
f_topic           Text
f_oper            Text
f_obj_id           Text
f_version          BigInt
f_generation       BigInt
f_status           Text
f_visible_at       Timestamptz
f_retried_count    BigInt
f_lease            BigInt
f_error            Nullable<Text>
f_created_at       Timestamptz
f_updated_at       Timestamptz
```

Forbidden:

- `diesel::sql_query` and `QueryableByName`;
- runtime table/column names;
- a table-erasing trait;
- proc-macro validation used instead of Diesel inference;
- fallback code that compiles without touching the table.

Compile-fail fixtures delete each required column and independently change each
column's type and nullability, covering the object-row contract and the entire
task-row contract. Missing tables also fail compilation. Runtime tests are not
accepted as proof of this contract.

## 8. ObjPool and ObjProm

`ObjPool` exposes only:

- `gen_slot(key, content_type, byte_len)`;
- `gen_url(key)`;
- `has(key)`;
- `del(key)`.

There is no `ObjPoolSpec`. R2 knows no object marker or business policy.

`ObjProm` owns actor-side durable work and is independent from
`ObjPromDefer<C>`:

- `reset_tasks()`;
- `claim_task()` across all topics;
- `complete_task(task)`;
- `retry_task(task, message)`;
- `mark_task_operator(task, message)`.

`ObjPromDefer<C>` owns transaction-side creation and does not extend
`ObjProm`:

- `defer_check(context, topic, key, visible_at)`;
- `defer_delete(context, topic, key)`.

`NormObjDept<Pool, Prom>` requires `Prom: ObjProm` only for construction and
actor operation. Each transaction-scoped Step implementation separately adds
`Prom: ObjPromDefer<C>` for its concrete context.

There is no transition enum and no `settle` method. Every actor transition is
fenced by task id, Processing status, and exact lease. A zero-row fencing
update means ownership was lost: it returns an explicit lost-lease outcome,
never success, and the actor performs no second transition for that attempt.
Remote Check/Delete failures remain retryable without a terminal count limit.

`defer_check` and `defer_delete` allocate and inspect their obligation in the
owner transaction. An existing row is accepted only when topic, operation,
object id, version, and generation are identical and its state is Pending or
Processing. A structurally different row, unknown state, or Operator row is an
error that rolls back the owner transaction. Completed is accepted only for an
immutable obligation that cannot recur; a recurring obligation allocates the
next generation in the same transaction. No conflict path silently drops a
debt.

The task table uses `f_topic`. Topic is routing metadata, not a dynamically
selected Diesel table.

## 9. One ObjActor

ObjDept owns exactly one event loop:

```text
NormObjDept
├── core
├── pool
├── prom
└── actor_desc
```

`ObjActor::new` immediately spawns the background task and returns one
`ObjActorDesc`. No item in the touched ObjDept path contains `start`.

`ObjActorDesc` directly owns cancellation and completion state. There is no
ActorInner, ActorHarn, ObjActorCtrl, descriptor array, or descriptor vector.

The first scheduler is intentionally serial:

```text
reset expired leases
claim globally oldest visible task
dispatch one task by topic
await its typed handler
apply one explicit task transition
repeat
```

The scheduler receives a raw fenced envelope after the row has mechanically
entered Processing. The envelope always retains task id and exact lease even
when topic, operation, status, version, or generation decoding fails. Decode
failure is therefore marked Operator through the same fence. Candidate
selection also consumes malformed active statuses; they cannot sit outside the
claim path forever. A lease that cannot be incremented is marked Operator
atomically and skipped so one corrupt row cannot block every topic.

Each claimed attempt has a cancellation-aware deadline. Remote HEAD/Delete
uses a shorter timeout, and the Processing lease duration is strictly longer
than the whole-attempt deadline. Attempt timeout leaves or restores retryable
debt for later claim. Descriptor shutdown cancels the in-flight attempt and
waits for the event loop, so a hung remote call cannot freeze all topics or
prevent shutdown.

Future topic-parallel scheduling may change only ObjActor internals. It must not
change actor count, NormObjDept, Harn, main, ObjPool, ObjProm, or use cases.

The generated exhaustive dispatch is:

```text
page_image  -> typed PageImage handler  -> t_page_image
user_avatar -> typed UserAvatar handler -> t_user_avatar
team_avatar -> typed TeamAvatar handler -> t_team_avatar
comic_cover -> typed ComicCover handler -> t_comic_cover
unknown/invalid envelope -> explicit operator task transition
```

This is a static match of monomorphic handlers, not a registry or dynamic table
selection.

## 10. Handler lifecycle

The generated handler knows only Check/Delete and mechanical object state.

Check:

- verified current version: complete task;
- pending current version and remote exists: verify exact version, complete;
- pending current version and remote absent: retire exact version, delete the
  physical key, complete;
- missing, retired, or stale: delete physical key, complete;
- future or invalid tuple: mark task operator;
- transient RDB/remote failure: retry indefinitely.

Delete:

- missing, retired, or stale: delete physical key, complete;
- pending or verified current version: mark task operator;
- future or invalid tuple: mark task operator;
- transient RDB/remote failure: retry indefinitely.

Remote HEAD/Delete runs outside transactions. Exact state changes use typed CAS
and re-read after a lost CAS.

## 11. `NormObjDept` composition

`impl_obj_dept!` consumes the local manifest and generates:

```text
NormObjDept<Pool, Prom>
```

It owns one RdbCore, injected Pool, injected Prom, and one ObjActorDesc. Concrete
R2/RDB Prom types never appear in macro input.

Its module-private parts constructor invokes `ObjActor::new` during
construction. There is no second launch phase. The generated actor receives
one closure whose owned future is the complete generated `dispatch`; the
closure clones the neutral core/pool handles into that future. This avoids a
public handler registry or a table-erasing handler trait.

The macro generates:

- Run/Step implementations for every marker;
- one typed topic-dispatch function;
- total ObjDept capability coverage;
- construction and shutdown of exactly one actor.

Remove the server `ObjRead` wrapper. Use cases invoke ObjDept opers directly.
Non-transaction reads state their actual capability as
`for<'a> Run<GetObjMeta<'a, Marker>, Error = ObjDeptError>` and/or
`for<'a> Run<GenObjUrl<'a, Marker>, Error = ObjDeptError>`; no unused context
type or unconstrained associated error remains to infer.
The use case awaits those opers and passes only resolved metadata/URL values to
View construction. Remove `PageObjView`; View never executes an oper and never
depends on an ObjDept adapter.

## 12. Harn, main, and chapter advancement

Move `HybNucl` from `harn.rs` to the transaction adapter area. Harn contains
only composition and accessors.

Production construction becomes:

```text
RdbCore
├── HybNucl
├── HybRepo
├── RdbObjProm
├── R2ObjPool
└── NormObjDept<R2ObjPool, RdbObjProm>
       └── ObjActor::new -> one ObjActorDesc
```

`RdbObjProm` may be a public concrete type so it can occur in the binary's Harn
type, but its fields and direct constructor remain private. A public production
factory in the server ObjDept adapter module constructs `RdbObjProm` internally
and returns `NormObjDept<R2ObjPool, RdbObjProm>`. Main never constructs or
receives actor-side Prom capabilities separately.

ObjActor never advances Chapter stages. Both page reservation paths atomically
defer `TryAdvanceRawProvideStage` through business Prom. Its handler:

1. locks/reads the Chapter and complete Page set in one transaction;
2. reads every newest PageImage through typed ObjDept metadata;
3. waits without consuming failure budget while any image is absent/pending;
4. completes RawProvide only when every newest PageImage is verified;
5. writes one workflow record in that transaction;
6. emits the completion effect after commit.

Business tasks remain in `t_local_message`; ObjDept Check/Delete tasks remain in
`t_obj_prom_task`.

Required tests cover multiple pending pages, partial upload, final-page
advancement, replacement versions, repeated delivery, and already-completed
chapters.

## 13. Migration boundary

After contracts are accepted, change only mechanical task routing naming and
the indexes required by global claiming:

- rename the task-table column to `f_topic` in the creation migration, and
  remove every `f_bind` reference from both creation and index migrations;
- define the poll index as
  `(f_status, f_visible_at, f_created_at, f_id)`;
- define the stuck index as `(f_status, f_updated_at, f_lease)`;
- keep all four object tables and `t_obj_prom_task` limited to their primary
  key, mechanically necessary `NOT NULL`/defaults, and ordinary indexes;
- add no foreign key, extra unique constraint, CHECK, database enum/domain,
  trigger, or partial index encoding fixed task/business policy;
- regenerate schema through Diesel only.

No migration changes before contract acceptance.

## 14. Implementation phases

### Phase 0: contract approval

1. Review this plan.
2. Rewrite `specs/obj-dept.md` to match the accepted plan.
3. Freeze macro inputs, trait methods, and the naming matrix.
4. Do not modify Rust until spec review has no blocker.

### Phase 1: macro proofs

1. Add `obj_inst!` compile-pass/fail fixtures.
2. Prove `objs_def!` emits the frozen callback ABI and rejects duplicate
   marker/table/topic/namespace entries.
3. Prove `impl_obj_dept!` consumes that manifest as complete items without
   global state, loose match-arm expansion, or erased futures.
4. Add the per-column missing/type/nullability schema compile-fail matrix.

### Phase 2: public contracts

1. Split `model::obj` by function.
2. Remove ObjPoolSpec.
3. Replace ObjPromBind with ObjPromDefer.
4. Replace settle with explicit task transitions.
5. Remove oper constructors and meaningless must-use attributes.

### Phase 3: typed adapters and schema

1. Generate all object helpers from one manifest.
2. Generate typed RDB Prom defer/claim/recovery/transition operations,
   including raw-envelope, collision, malformed-row, and lost-lease outcomes.
3. Rename task routing from bind to topic.
4. Apply accepted migration edits and regenerate schema.
5. Run schema compile-fail fixtures before server cutover.

### Phase 4: single actor

1. Replace per-object actors with one serial ObjActor.
2. Generate the exhaustive typed topic dispatcher with branch-local awaits.
3. Make ObjActor::new spawn directly.
4. Reduce control state to one ObjActorDesc.
5. Port Check/Delete fencing and lifecycle tests.

### Phase 5: composition and use-case cutover

1. Generate NormObjDept with injected Pool/Prom.
2. Move HybNucl out of harn.rs.
3. Remove ObjRead.
4. Replace every oper constructor with obj_inst!.
5. Remove non-use `crate::` paths in the touched scope.
6. Wire one actor descriptor through the production factory and Harn.
7. Replace ObjRead/PageObjView with explicit Run bounds and resolved View
   inputs.

### Phase 6: business lifecycle proof

1. Preserve chapter advancement in business Prom.
2. Add full last-page advancement tests.
3. Retest replacement, deletion, archive, avatar, and cover paths.
4. Verify business and ObjDept task-table isolation.

### Phase 7: audits and validation

1. Delete obsolete names rather than aliasing them.
2. Run focused macro, ObjDept, actor, RDB, and use-case tests.
3. Run schema and disposable-DB migration validation.
4. Run repository CI only after focused checks pass.
5. Obtain independent macro, actor, Diesel, dependency, and lifecycle reviews.

## 15. Required source audits

The touched path contains none of:

```text
ObjRemote
ObjAdapter
RdbObjBind
ActorHarn
ActorInner
ObjActorCtrl
RdbObjDept
ObjPoolSpec
ObjPromBind
BindRow
bind_check
bind_delete
ObjActorSettle
ObjPromSettle
settle
start
get_upload_slot
#[obj_dept(...)]
diesel::sql_query
QueryableByName
```

Structural audits prove:

- one production `objs_def!` invocation;
- one ObjActor construction and one ObjActorDesc field;
- no descriptor Vec/array;
- no ObjDept oper inherent constructors;
- no direct `_m` initialization in server code;
- no non-use `crate::` paths in touched production code;
- every required table column is referenced by Diesel DSL;
- duplicate marker/table/topic/namespace manifests fail compilation;
- malformed claimed rows and lease overflow cannot block global progress;
- every in-flight attempt and remote call has the specified timeout/cancel
  boundary;
- every touched Rust file remains below the limit.

## 16. Stop conditions

Stop and revise before implementation if any phase requires:

- more than one ObjActor or one actor per topic;
- a runtime handler registry or dynamically selected table;
- a table-erasing operation trait;
- a manifest macro expanding to loose match arms or an erased future;
- concrete Pool/Prom types in `impl_obj_dept!` input;
- an oper inherent constructor;
- a compatibility alias for a rejected name;
- chapter business knowledge inside ObjActor;
- ObjDept Check/Delete work in business Prom;
- remote HEAD/Delete inside an RDB transaction;
- a conflict path that treats a different or Operator task as satisfied;
- an unbounded actor attempt or remote call;
- mutable business policy enforced by new database constraints.

## 17. Completion criteria

The correction is complete only when:

- every latest to-fix item maps to a passing source audit;
- one object manifest drives typed helpers and the single actor dispatcher;
- `obj_inst!` is the only server construction path for PhantomData opers;
- one NormObjDept owns one actor descriptor;
- `ObjActor::new` is the only actor launch point;
- all task transitions use explicit action names;
- Font ObjDept infrastructure registration changes only marker, standard
  schema, and one manifest entry;
- PageImage, avatar, and cover lifecycles remain complete;
- final-page verification advances RawProvide exactly once through business
  Prom;
- compile-fail schema fixtures and runtime lifecycle tests pass;
- independent reviewers report no blocking issue.
