---
name: gotchas
description: Recurring mistakes and pitfalls in poprako-r. Each entry is a pattern that was actually screwed up at least once. Consult before committing any non-trivial change.
---

# Gotchas

- `*QueryTransactional::delete` returns `DomainResult<()>`, never leaks orphaned data through the return type
- Side-effect cleanup (avatar files, etc.) belongs inside the `complex` function, not in the usecase
- No nested `if` — use `match` on a tuple to handle multi-condition branching in a single arm
- Data-object types name the aggregate root first, verb after: `TeamAvatarReserveParams` not `ReserveTeamAvatarParams`
- Every `&mut self` transactional method that does `SELECT … FOR UPDATE` must be suffixed `_excluded`
- `Option<String>`: never call `.is_empty()` / `.unwrap_or_default()` on it; never coerce `None` to `""` for external APIs
- Entity `From`: pass `Option` columns through as-is (`field: v.f_foo`), never `.unwrap_or_default()`
- Test assertion: `assert_eq!(aggr.field, Some("val".into()))` not `assert_eq!(aggr.field, "val")`
- Don't `.clone()` an owned value when you can move it; only clone behind shared references that must survive
- After bulk `sed -i` across files, always `rg` for doubled suffixes or missed spots
- Changing a field from `T` to `Option<T>`: grep the field name across all of `src/` and update every constructor, assignment, assertion, and `From` impl
- Query layer param names: the parameter name must reflect the type's role — if the type is `*Update` use `update`, if `*Form` use `form`. Never use a generic placeholder like `input`. This applies to trait signatures, free function signatures, impl blocks, and all local variables at call sites.
