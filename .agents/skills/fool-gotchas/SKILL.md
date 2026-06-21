---
name: fool-gotchas
description: Recurring mistakes and pitfalls in poprako-r made by AIs. Each entry is a pattern that was actually screwed up at least once. Consult before committing any non-trivial change.
---

# Gotchas

- `*RepoTransactional::delete` returns `DomainResult<()>`, never leaks orphaned data through the return type
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
- **Handler/usecase naming inconsistency fix direction**
  What was done wrong: saw handler `list` calling usecase `list_infos` with mismatched names, assumed the handler name was authoritative, and renamed `list_infos` → `list` in the usecase.
  What is correct: the usecase name is authoritative. The handler must follow the usecase. Rename handler `list` → `list_infos` and update router + openapi references to match.
- **Never start a rename operation without first confirming direction**
  What was done wrong: user said "handler and usecase must have the same name", started renaming the usecase without first asking which name was the canonical one, and got the direction backwards.
  What is correct: when the user says "X is the correct name", X is canonical — everything else must be renamed to match X. If the user does not specify a direction, ask first, then act.
- **Variable names must reflect what the value represents, never the layer boundary**
  What was done wrong: in an HTTP handler, named the constructed usecase params struct `usecase_params`, encoding which layer it is passed to rather than what data it carries.
  What is correct: name the variable after the params type's role. `MemberListParams` → `list_params`, `MemberMineParams` → `mine_params`. The existing query-layer rule ("Never use a generic placeholder like `input`") applies equally to handler local variables.
- **HTTP query parameter structs belong in the usecase data_object layer, never in the handler file**
  What was done wrong: defined `MemberListQuery`, `ListMyMembersQuery`, `MemberMineQuery` directly in the handler file (`src/api/http/handler/member.rs`) with local `#[derive(Deserialize, IntoParams)]`, mixing API-layer struct definitions with handler functions.
  What is correct: move all `*Query` structs into `src/usecase/data_object/member.rs` alongside the `*Params` types. The handler file imports them and contains only handler functions and their `#[utoipa::path]` annotations. Same rule for team, workset, and all other resource handlers.
- **Data-object query structs must follow the same aggregate-first naming as params**
  What was done wrong: named the HTTP query struct `ListMyMembersQuery` (verb-first), violating the existing rule "Data-object types name the aggregate root first, verb after". The matching params type was already correctly named `MembersListMineParams`.
  What is correct: `MembersListMineQuery` — aggregate root `Members` first, descriptor `ListMine` after, suffix `Query` last. Same rule as params (`*Params`), replies (`*Reply`), and bases (`*Base`).
