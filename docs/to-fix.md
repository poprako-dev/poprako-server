# ObjDept feedback reconciliation

Reviewed against the working tree on 2026-10-25. This ledger preserves all
22 original requests in their original order. Resolved items cite current
source, rather than claiming release validation has passed. The current
contract is [the ObjDept specification](../specs/obj-dept.md).

## Original requests and disposition

| Request | Current evidence and disposition |
| --- | --- |
| 1. Define the pool trait inside ObjDept, implement it with R2, and remove the server `ObjRemote` port. | Resolved: `ObjDeptPool`/`ObjDeptPoolView` live in `poprako-obj-dept/src/pool.rs`; the R2 implementation lives in `src/part_impl/obj_dept/r2_impl.rs`. |
| 2. Name event loops `*Actor` and control handles `*ActorDesc`, replacing `ActorHarn`. | Resolved for ObjDept: `ObjDeptActor` and `ObjDeptActorDesc` in `poprako-obj-dept/src/actor.rs`. |
| 3. Remove misleading `ActorInner`. | Resolved in ObjDept: the actor directly owns `prom`/`handler`, and its descriptor owns cancellation/join handles. |
| 4. Name effectful upload generation with `gen`, rather than `get_upload_slot`. | Resolved: `GenObjSlot`, `GenObjSlots`, and pool `gen_slot`. |
| 5. Name the error after ObjDept. | Resolved: `ObjDeptError` in `poprako-obj-dept/src/rest.rs`. |
| 6. Support future kinds such as Font through macros. | Generation seam exists: `objs_def!` and `impl_obj_dept!`; macro test `generated_names_are_readable_and_read_views_cover_every_marker` includes `FontFile`. A real Font feature remains separate product work. |
| 7. Prove missing tables and columns fail compilation. | Typed implementation exists; explicit negative compilation coverage remains FIX-01. |
| 8. Name phantom fields only `_m`. | Resolved in ObjDept identity and operation fields in `key.rs` and `oper.rs`. |
| 9. Replace untyped `rdb_impl` indirection with direct Diesel schema inference. | Concrete table paths, typed expressions and `Selectable` projections are emitted by `poprako-obj-dept-macro/src/object/rdb_entry.rs`. The remaining `rdb_impl` module provides shared typed support; the name alone does not establish the old defect. Compiler rejection evidence remains FIX-01. |
| 10. Split models by function instead of a nested generic `obj` module. | Resolved: `model.rs` declares `alloc`, `meta`, `record`, `slot`, `task`, and `url`. |
| 11. Replace ambiguous `ObjPoolSpec`. | Resolved: `ObjSlotSpec` describes upload requirements; `ObjUrlSpec` selects URL renditions. |
| 12. Remove `settle` from types and variables. | Resolved in inspected ObjDept source: explicit `persist_action`, `complete_task`, `retry_task`, and `mark_task_operator` names. |
| 13. Remove unclear settle/Bind machinery. | Current dispatch uses `ObjKind` markers, generated typed table modules and explicit task actions. |
| 14. Keep `HybNucl` definitions out of `harn.rs`. | Resolved: `src/harn.rs` stores generic ports; `src/main.rs` composes `HybNucl`. |
| 15. Do not store ObjActor as an array. | Resolved: `src/main.rs` owns one `ObjDeptActorDesc`; its supervisor joins claim and maintenance loops. |
| 16. Forbid start-named APIs and spawn tasks in `new`. | No start-named ObjDept API remains, but `new` constructs without spawning and `run_detached` spawns. Lifecycle disposition remains FIX-02. |
| 17. Do not give operation descriptors constructors. | Policy conflict: `oper.rs` has `new` constructors; the later [remediation plan](../plans/obj-dept-remediation.md) explicitly requested these to replace `obj_inst!`. Preserve as FIX-03 pending confirmation of the governing decision. |
| 18. Name the total implementation `NormObjDept`. | Resolved: `src/part_impl/obj_dept.rs` defines `NormObjDept`; `RdbObjDeptProm` names only the database queue adapter. |
| 19. Replace repeated `obj!` declarations with `objs_def!`. | Resolved: the server manifest uses `objs_def!`. |
| 20. Avoid `crate::` calls outside imports. | Review remains FIX-04: qualified paths remain in tests and macros, whose hygiene requirements need a separate disposition. |
| 21. Remove unnecessary `must_use` attributes. | Review remains FIX-05: attributes remain on constructors, helpers and detached actor return values. |
| 22. Name the server part `obj_dept`, rather than `obj`. | Resolved: `src/part/obj_dept.rs` and `src/part_impl/obj_dept.rs`. |

## Open tasks and acceptance

- [ ] **FIX-01 — Prove Diesel schema rejection.** Add compile-fail coverage for
  a declaration referencing a missing table, a missing required column, and an
  incompatible column type. Retain the successful typed fixture in
  `poprako-obj-dept/tests/rdb_obj.rs`. Acceptance: invalid fixtures fail through
  concrete Diesel schema/type checking; valid operations compile without raw
  SQL or string-selected tables. Existing macro token tests do not establish
  all three failure cases.
- [ ] **FIX-02 — Resolve actor construction lifecycle.** Reconcile the original
  auto-spawn requirement with `ObjDeptActor::new`/`run_detached` and composition
  in `src/main.rs`. Acceptance: the active spec records the governing contract;
  constructor spawning is implemented with cancellation/shutdown regression
  coverage, or the original requirement is explicitly superseded by the user.
- [ ] **FIX-03 — Resolve operation construction policy.** Reconcile the original
  prohibition with the later remediation's associated-constructor requirement
  and private `_m` fields. Acceptance: one documented decision governs all
  descriptors and ordinary call sites compile without opaque macro navigation.
  Do not undo the later implementation merely from this historical checklist.
- [ ] **FIX-04 — Review qualified calls.** Inspect ordinary handwritten runtime
  code and tests, including `poprako-obj-dept/src/tests.rs`, separately from
  macro bodies. Acceptance: ordinary calls follow import conventions; retained
  `$crate` and emitted absolute dependency paths have a hygiene rationale.
- [ ] **FIX-05 — Review `must_use` individually.** Review `oper.rs`,
  `model/url.rs`, `rest.rs`, `prom.rs`, `actor.rs`, `rdb_impl.rs`, `key.rs`, and
  the server R2 adapter. Acceptance: remove redundant annotations and justify
  retained lost-result contracts or lint requirements; canonical Rust checks
  pass without weakening linters.

Resource-limit and value/model audits remain in [the maintenance backlog](todo.md).
