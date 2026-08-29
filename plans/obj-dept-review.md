# ObjDept Review Record

## Review target

The reviewed slice is limited to the static PageImage declaration, total
server object, typed latest-state access, dedicated task table, and Check/Delete
actor. Existing Page traffic is not cut over.

## Required conclusions

### Dependency direction

- `poprako-rdb-core` contains neutral mechanics only;
- `poprako-obj-dept` depends downward on those mechanics under its optional
  feature;
- `poprako-server` supplies concrete PageImage behavior;
- the server composes one non-generic `RdbObjDept`.

### Static dispatch

- one `#[obj_dept(Image, t_page_image)]` declaration exists;
- the generated code uses a concrete Diesel table module;
- actors are monomorphized by bind and do not use a runtime handler map;
- the macro does not generate runtime policy.

### Persistence

- `t_page_image` stores only the latest active tuple or detached version;
- `t_obj_prom_task` is separate from legacy Prom;
- no raw SQL or untyped row decoding exists in ObjDept paths;
- database constraints remain mechanical.

### Reliability

- state change and task creation share one owner transaction;
- Check cannot verify a stale version;
- Delete cannot delete a current version;
- compare-and-set misses are re-read;
- remote side effects occur outside transactions;
- leases fence settlement and expired leases are reclaimed;
- retry count never removes unresolved deletion debt;
- restart preserves unfinished tasks.

### Scope control

- no inventory scan or late-object repair exists;
- no activation or copy-journal state exists;
- no existing Page producer uses the new path;
- legacy Page fields remain until a separately reviewed cutover.

## Evidence checklist

- [x] format check passes;
- [x] all-feature compile passes;
- [x] ObjDept crate tests pass;
- [x] server library tests pass;
- [x] custom linters pass;
- [x] checked-in CI passes;
- [x] typed-Diesel scan is clean;
- [x] all touched Rust files remain under the project line limit;
- [x] read-only dependency, lifecycle, and migration reviews have no blocker.

## Deferred questions

These are intentionally not answered by this implementation:

- how existing verified Page objects move to the new key grammar;
- when old Page image columns are removed;
- how the legacy Page message backlog is drained;
- whether late-object inventory repair is ever required.

They require a separate plan because choosing them changes production data and
remote object ownership.
