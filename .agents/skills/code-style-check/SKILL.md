---
name: code-style-check
description: All 70 formatting/naming/structural rules across every poprako-r layer. Run each rule as a discrete check.
---

# Code Style Check — 70 Rules

---

## A. Comments & Language (3 rules)

1.  **English-only comments** — all `//`, `/* */`, `///`, `//!` must be in English.
2.  **No Go references** — no Go file paths, function names, or patterns in comments.
3.  **No per-test comments** — no `//` directly above `#[test]` / `#[tokio::test]`; put all explanation in the module-level test-case description list.

---

## B. Abbreviations (3 rules)

4.  **No abbreviations in code identifiers** — type names, field names, function names, parameter names, and variable names must be full English words.
5.  **Allowed pre-existing abbreviations**: `harn`, `conn`, `txn`, `id`, `qid`, `aggr` (suffix only), `sadmin` (field only), `i18n` (module only).
6.  **Forbidden patterns**: `_val` suffix, `_filter` suffix, `cnt`, `ws`/`ws_id`, `desc`, `upd`, `cre`, `resv`, `new_val`, `offset_val`, `limit_val`.

---

## C. Imports (5 rules)

7.  **No wildcard imports** — except `use super::*` in test modules and Diesel/Schema preludes.
8.  **No non-leaf braces** — `use a::b::{c, d}` ✅ / `use a::{b, c::d}` ❌.
9.  **Import concrete items** — every type/trait/function used in code bodies is imported by name; no long `crate::...` paths in signatures or expressions.
10. **API handler exception** — handler files import `use crate::usecase;` and call `usecase::team::create(...)` with the `usecase::` prefix.
11. **Import order**: std/external crates → blank line → crate imports.

---

## D. Macros (3 rules)

12. **Attribute macros** — `use` import + bare name: `use tracing::instrument;` → `#[instrument]`.
13. **Derive macros** — `use` import + bare name: `use serde::Serialize;` → `#[derive(Serialize)]`.
14. **tracing event macros** — fully qualified at call site (`tracing::error!(...)`), never imported.

---

## E. Format Strings (2 rules)

15. **No inline captures** — `format!("{ident}")` ❌, `format!("{}", ident)` ✅.
16. **tracing fields** — `tracing::error!(key = %val, "message")` ✅, no interpolated values in message string.

---

## F. Visibility & Fields (3 rules)

17. **Only `pub` or private** — no `pub(crate)`, `pub(super)`, etc.
18. **Data containers have `pub` fields** — aggregates, value objects, entity structs.
19. **Logic-carrying types have private fields** — query handles, harness fields, effect sinks.

---

## G. Ownership & Construction (3 rules)

20. **Constructor over struct literal** — when `new()` exists (Aspect, events-carrying aggregates), use it. Struct literals only for data-only types without constructors.
21. **Borrow over clone** — pass references when possible; never clone at a value's final use.
22. **Aspect builder** — always `Aspect::new(now).field(val)`, never struct literal for Aspect.

---

## H. #[instrument] Placement (5 rules)

23. **No instrument on constructors** — `new`, `generate_id`, `From`, `Default`.
24. **No instrument on domain model** — any function under `src/domain/`.
25. **No instrument on harness** — `src/harness.rs` impl blocks.
26. **No instrument on RepoTransactional impl** — pure delegation.
27. **Allowed on**: usecase functions, infra query free functions, infra `Query` impl, infra external services, API handlers.

---

## I. Error Construction (7 rules)

28. **Expected: `DomainError::expected_argument(trl("error-xxx"))`** — never struct literal.
29. **Expected: `DomainError::expected_authentication(trl("error-xxx"))`**.
30. **Expected: `DomainError::expected_conflict(trl("error-xxx"))`**.
31. **Unrecoverable: `DomainError::unrecoverable(format!("[Struct::method] ..."))`** — must have `[Struct::method]` prefix.
32. **No hardcoded language** in error messages — always use `trl("error-xxx")`.
33. **No `[Struct::method]` prefix on Expected** messages — that prefix is only for Unrecoverable.
34. **Diesel NotFound**: always `.optional()?` → `.ok_or_else(|| DomainError::expected_argument(trl(...)))`.

---

## J. i18n (2 rules)

35. **Literal i18n keys in kebab-case**. Expected error messages use `error-`; non-error messages keep their domain prefix, such as `mail-`.
36. **Every literal `trl(...)` / `trl_kv(...)` key must exist in both** `locales/en-US/main.ftl` and `locales/zh-CN/main.ftl`.

---

## K. UFCS (Universal Function Call Syntax) (2 rules)

37. **UFCS for all trait method calls on harness** — `Trait::method(harn, args...)` ✅, `harn.method(args...)` ❌.
38. **UFCS inside `transaction_scoped` closures** — `Trait::method(query, args...)` on the `query` parameter.

---

## L. Usecase Layer (4 rules)

39. **Composite trait bounds**: `H: Query + ImageGet + Send + Sync` (never individual `UserQuery`).
40. **Transactional**: `H: Clone + Transactional + Send + Sync`.
41. **`.boxed()` not `Box::pin`** — `use futures_util::FutureExt as _;` then `.boxed()`.
42. **`transaction_scoped` only when cross-aggregate atomicity needed** — single-row ops call `*Query` directly.

---

## M. Type Annotations (2 rules)

43. **`let` annotation over turbofish** — `let row: UserRow = ...` ✅, `.first::<UserRow>(conn)` ❌.
44. **No turbofish anywhere in Diesel query chains** — `.first()`, `.get_result()`, `.load()` never carry type parameters.

---

## N. Domain Aggregate Layer (6 rules)

45. **Suffixes**: read-model=`Aggr`, create-input=`Form`, PUT-input=`Update`, PATCH-input=`Patch`.
46. **No `Cre` suffix** — use `Form`.
47. **`Form` ID via `Aggr::generate_id()`**; `Update`/`Patch` ID is caller-provided.
48. **No `new()` unless aggregate has private `events` field**.
49. **All fields `pub` except `events`** (private, placed last).
50. **`From<EntityRow>` in entity module**, uses struct literal.

---

## O. Domain Query Trait Layer (6 rules)

51. **One file per aggregate** under `src/domain/query/`.
52. **Two traits**: `{Aggr}Query` (`&self`) and `{Aggr}RepoTransactional` (`&mut self`).
53. **`*Query` = single-row ops, no transaction**.
54. **`*RepoTransactional` = cross-aggregate atomicity needed**.
55. **Never same method on both traits**.
56. **Reference params in trait signatures**: `&str`, `&Form` (not owned).

---

## P. Infra Query Entity Layer (4 rules)

57. **Suffixes**: `Row`=`Queryable+Selectable`, `Entry`=`Insertable`, `Aspect`=`AsChangeset`.
58. **Aspect: `new(updated_at)` + builder** — each `Option` field gets a builder method.
59. **Database columns use `f_` prefix** — `f_id`, `f_nickname`.
60. **`From<EntityRow>` in entity module**, not in query logic file.

---

## Q. Infra Query Impl Layer (4 rules)

61. **Free functions take `conn: &mut AsyncPgConnection`** — `pub async fn`.
62. **`RdbQuery` impl uses `submit_query!`** — `submit_query!(self.pool, free_fn, args...)`.
63. **`RdbRepoTransactional` impl delegates directly** — `free_fn(self.conn, args).await`.
64. **INSERT uses `*Entry` struct** — `.values(&entry)`, never inline tuples.

---

## R. Memory Mock Layer (1 rule)

65. **`MemoryMockQuery` impls `*Query`; `MemoryMockRepoTransactional` impls `*RepoTransactional`**.

---

## S. API Handler Layer (4 rules)

66. **`Accept as _`** — anonymous import, return `.accept(StatusCode::...)`.
67. **`?` propagation** — never manually match usecase errors.
68. **`HttpError` imported and bare** — `body = HttpError` in `#[utoipa::path]`, never `crate::api::http::result::HttpError`.
69. **Handler name matches usecase function name**.

---

## T. Test Modules (2 rules)

70. **`use super::*` must be first import** in every test module (after test-case descriptions).
71. **Test-case descriptions before imports**: `// name(target)(positive|negative): description`.

---

## Quick Commands

```bash
# 4-6: abbreviations
rg -nE '\bws\b|\bws_\b|\bcnt\b|_val\b|_filter\b|\bdesc\b|\bupd\b|\bcre\b|\bresv\b' --include='*.rs' src/

# 7-9: fully-qualified paths in code bodies
rg -n 'crate::' --include='*.rs' src/ | grep -v 'use ' | grep -v '///' | grep -v '#\['

# 12-14: macro violations
rg -n '#\[tracing::instrument\]|#\[async_trait::async_trait\]' --include='*.rs' src/

# 15-16: inline format captures
rg -n '\{[a-z_][a-z0-9_]*[:?]?\}' --include='*.rs' src/ | grep -v 'utoipa' | grep -v '//'

# 43-44: turbofish in query chains
rg -n '\.first::|\.get_result::|\.load::|\.execute::' --include='*.rs' src/

# 23-27: instrument on constructors/domain/harness
rg -n '#\[instrument\]' --include='*.rs' src/domain/ src/harness.rs
```
