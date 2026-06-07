---
name: implement-fullchain-spec
description: >
  Mandatory checklist for implementing a usecase end-to-end across ALL layers
  of poprako-r (domain model → query trait → infra query entity + impl +
  memory mock → usecase data objects + usecase → i18n → API handler →
  router → TestHarness).  Covers every mistake category observed in actual
  implementation: naming precision, Query vs QueryTransactional boundary,
  entity suffix rules, UFCS mandate, composite trait bounds, Box::pin ban,
  transaction discipline, TestHarness forwarding, i18n, and the full
  implementation sequence.  Use whenever implementing, extending, or
  completing any usecase chain listed in docs/refactor/usecase-checklist.md.
---

# Full-Chain Implementation Specification

Every rule below comes from a concrete mistake that was caught in review.
They are ordered by the layer where the mistake most often occurs, but
many rules span multiple layers — read them all before implementing.

---

## 1. Domain Model Layer (`src/domain/model/aggr/`)

### 1.1 No abbreviations in new identifiers

All new type names, field names, and function names must use **full English
words**.  Existing abbreviations in the codebase (`qid`, `aggr`, `harn`,
`txn`, `sadmin`) were chosen by the project author — do not copy new
abbreviations from the Go reference.

| ❌ Wrong | ✅ Correct |
|---|---|
| `UserUpd` | `UserInfoUpdate` |
| `ResvAvatarArgs` | `ReserveAvatarArgs` |
| `upd` (variable) | `input` / `update_form` |

### 1.2 Names must precisely describe what is affected

If an Update aggregate only touches profile info (nickname, qid) and NOT
password, the name must make that scope visible.

```rust
// ❌ Wrong — "UserUpdate" implies a full user PUT (including password).
//           The aggregate-definition-spec uses "Update" as the category
//           suffix, but the PREFIX must describe the actual scope.
pub struct UserUpdate { pub qid: String, pub nickname: String }

// ✅ Correct — "Info" scopes the update to profile fields only.
pub struct UserInfoUpdate { pub qid: String, pub nickname: String }
```

### 1.3 Follow aggregate-definition-spec for suffix conventions

| Category | Suffix | ID source |
|---|---|---|
| Read-model | `Aggr` | From entity row |
| Input: Form | `Form` | `*Aggr::generate_id()` |
| Input: Update | `Update` | Caller provides |

The suffix is mandatory (`*Update`), the prefix describes scope
(`UserInfoUpdate` not `UserUpdate`).

---

## 2. Domain Query Trait Layer (`src/domain/query/`)

### 2.1 Query vs QueryTransactional: the boundary

**This is the most common mistake.**  Every new method must go on the
correct trait:

| Trait | `self` type | When to use |
|---|---|---|
| `UserQuery` | `&self` | Single-row read or single-row write. No cross-aggregate atomicity needed. |
| `UserQueryTransactional` | `&mut self` | Must run inside a transaction because the usecase needs atomic writes across multiple aggregates or tables. |

```rust
// ✅ UserQuery — single-row operations, no transaction needed
pub trait UserQuery {
    async fn get_by_id(&self, id: &str) -> DomainResult<UserAggr>;
    async fn get_credentials_by_qid(&self, qid: &str) -> DomainResult<UserCredential>;
    async fn prefill_avatar_key(&self, id: &str, key: &str) -> DomainResult<()>;
    async fn mark_avatar_uploaded(&self, id: &str) -> DomainResult<()>;
    async fn touch_last_active(&self, id: &str) -> DomainResult<()>;
}

// ✅ UserQueryTransactional — only when cross-aggregate atomicity is required
pub trait UserQueryTransactional {
    async fn create(&mut self, form: &UserForm) -> DomainResult<UserAggr>;
    async fn update_info(&mut self, input: &UserInfoUpdate) -> DomainResult<UserAggr>;
}
```

**Deciding where a new method goes**: ask "does the usecase that calls this
method need to atomically write to more than one aggregate/table?" If no,
it goes on `UserQuery`.  Even if the Go reference uses a transaction (for
oss_msg or other features not yet ported), the Rust version starts on
`UserQuery` until the cross-aggregate dependency actually exists.

### 2.2 Use composite traits in where bounds (applies in usecase layer)

Every usecase's `where` clause must use composite traits, never individual
query traits:

```rust
// ✅ Correct
pub async fn get_info<H>(harn: &H, id: &str) -> UseCaseResult<UserBase>
where
    H: Query + ImageGet + Send + Sync,
{ ... }

// ❌ Wrong — individual query trait
pub async fn get_info<H>(harn: &H, id: &str) -> UseCaseResult<UserBase>
where
    H: UserQuery + ImageGet + Send + Sync,
{ ... }
```

`Query = UserQuery + TeamQuery + SystemMailQuery`.  Even if the usecase
only needs `UserQuery` today, bounding on `Query` prevents future churn.

---

## 3. Infra Query Entity Layer (`src/infra/query/entity/`)

### 3.1 Entity struct naming: Entry / Row / Aspect

The `query-infra-spec` defines strict suffixes.  Do not invent ad-hoc names.

| Suffix | Purpose | Diesel derive |
|---|---|---|
| `*Entry` | INSERT only | `Insertable` |
| `*Row` | SELECT / read | `Queryable`, `Selectable` |
| `*Aspect` | PATCH (partial update) | `AsChangeset` |

```rust
// ❌ Wrong — uses Entry suffix for an AsChangeset struct
#[derive(AsChangeset)]
pub struct UserUpdateEntry { ... }

// ✅ Correct — AsChangeset always uses Aspect suffix
#[derive(AsChangeset)]
pub struct UserAspect { ... }
```

---

## 4. Infra Query Layer (`src/infra/query/`)

### 4.1 Query impl vs QueryTransactional impl

Methods on `UserQuery` (`&self`) are implemented on `RdbQuery` using
`submit_query!`:

```rust
#[async_trait]
impl UserQuery for RdbQuery {
    async fn get_by_id(&self, id: &str) -> DomainResult<UserAggr> {
        submit_query!(self.pool, get_by_id, id)
    }
    async fn prefill_avatar_key(&self, id: &str, key: &str) -> DomainResult<()> {
        submit_query!(self.pool, prefill_avatar_key, id, key)
    }
    async fn mark_avatar_uploaded(&self, id: &str) -> DomainResult<()> {
        submit_query!(self.pool, mark_avatar_uploaded, id)
    }
    async fn touch_last_active(&self, id: &str) -> DomainResult<()> {
        submit_query!(self.pool, touch_last_active, id)
    }
}
```

Methods on `UserQueryTransactional` (`&mut self`) are implemented on
`RdbQueryTransactional` by delegating to the free function with
`self.conn`:

```rust
#[async_trait]
impl<'c> UserQueryTransactional for RdbQueryTransactional<'c> {
    async fn create(&mut self, form: &UserForm) -> DomainResult<UserAggr> {
        create(self.conn, form).await
    }
    async fn update_info(&mut self, input: &UserInfoUpdate) -> DomainResult<UserAggr> {
        update_user(self.conn, input).await
    }
}
```

### 4.2 Free functions: `submit_query!` for Query, direct conn for Transactional

Free functions for `UserQuery` methods use `submit_query!` (the pool
allocates a connection).  Free functions for `UserQueryTransactional`
methods take `conn: &mut AsyncPgConnection` directly.

---

## 5. Infra Memory Mock Layer (`src/infra/query/memory_mock/`)

### 5.1 Mirror the Query/QueryTransactional split

`UserQuery` methods → `impl ... for MemoryMockQuery`
`UserQueryTransactional` methods → `impl ... for MemoryMockQueryTransactional`

Never put a `UserQuery` method on `MemoryMockQueryTransactional` or vice
versa.

### 5.2 Tests: use `.boxed()` not `Box::pin`

Every test that uses `transaction_scoped` must use `.boxed()`:

```rust
mock.transaction_scoped(|txn| {
    async move {
        UserQueryTransactional::create(txn, &form).await.unwrap();
        Ok(())
    }
    .boxed()
})
.await
.unwrap();
```

Tests for `UserQuery` methods call the mock directly without a transaction:

```rust
UserQuery::prefill_avatar_key(&mock, "user-1", "key.png").await.unwrap();
```

---

## 6. Usecase Layer (`src/usecase/`)

### 6.1 UFCS for ALL trait method calls on harness

**Every** trait method called on `harn` must use UFCS. Direct method calls
are forbidden everywhere.

```rust
// ✅ Correct — UFCS always
UserQuery::get_by_id(harn, id).await?;
UserQuery::touch_last_active(harn, id).await?;
ImagePut::put_signed(harn, &key).await?;
Transactional::transaction_scoped(harn, move |query| { ... }).await?;

// ❌ Wrong — direct method call
harn.get_by_id(id).await?;
harn.touch_last_active(id).await?;
harn.put_signed(&key).await?;
harn.transaction_scoped(move |query| { ... }).await?;
```

Inside `transaction_scoped` closures, use UFCS on `query`:

```rust
UserQueryTransactional::create(query, &form).await?;
MemberInvitationQueryTransactional::get_by_code_ex(query, code).await?;
```

### 6.2 Composite traits in where bounds

Use `Query` (not `UserQuery`), `Transactional` (bundles all mutable
traits):

```rust
// Read-only usecase
where H: Query + TokenSign + Send + Sync

// Transactional usecase
where H: Clone + Transactional + Send + Sync

// Reads + transaction
where H: Query + Clone + Transactional + Send + Sync
```

### 6.3 Async blocks: `.boxed()` only

Never write `Box::pin(async move { ... })`.  Import `futures_util::FutureExt as _`
and use `.boxed()`:

```rust
use futures_util::FutureExt as _;

Transactional::transaction_scoped(harn, move |query| {
    async move {
        ...
        Ok(value)
    }
    .boxed()
})
.await?;
```

### 6.4 Transactions only when atomicity is needed

A usecase wraps logic in `transaction_scoped` **only** when it must
atomically write to multiple aggregates/tables.  Single-row operations
call `UserQuery` methods directly on `harn`.

```rust
// ❌ Wrong — single-row update wrapped in transaction
pub async fn touch_last_active<H>(harn: &H, id: &str) -> UseCaseResult<()>
where H: Clone + Transactional + Send + Sync,
{
    Transactional::transaction_scoped(harn, move |query| { ... }).await?;
    Ok(())
}

// ✅ Correct — direct call, no transaction
pub async fn touch_last_active<H>(harn: &H, id: &str) -> UseCaseResult<()>
where H: Query + Send + Sync,
{
    UserQuery::touch_last_active(harn, id).await?;
    Ok(())
}
```

### 6.5 Current-user usecases accept `UserToken` by value

Usecases that act on behalf of the authenticated user must accept
`UserToken` (owned), not `&UserToken` and never `curr_uid: &str`.

The API handler receives `UserToken` from request extensions via
`Extension(user_token): Extension<UserToken>` and passes it **by value**
to the usecase.  The usecase then extracts `.user_id` from the owned
token — no cloning needed.

```rust
// ✅ Correct — owned UserToken
pub async fn update_info<H>(
    harn: &H,
    token: UserToken,
    params: UserInfoUpdateParams,
) -> UseCaseResult<()> { ... }

pub async fn reserve_avatar<H>(
    harn: &H,
    token: UserToken,
    params: ReserveAvatarParams,
) -> UseCaseResult<ReserveAvatarReply> { ... }

pub async fn mark_avatar_uploaded<H>(
    harn: &H,
    token: UserToken,
) -> UseCaseResult<()> { ... }

// ❌ Wrong — &UserToken or curr_uid: &str
pub async fn update_info<H>(harn: &H, curr_uid: &str, ...) -> ... { }
pub async fn update_info<H>(harn: &H, token: &UserToken, ...) -> ... { }
```

**Why**: Passing `UserToken` by value makes the usecase own the identity
and avoids lifetime coupling with the request.  The handler already has
an owned `UserToken` from the middleware — passing it in moves it to the
usecase with zero overhead.

---

## 7. Usecase Data Objects (`src/usecase/data_object/`)

### 7.1 Follow the same naming rules

No abbreviations, precise names.  Params and Reply structs mirror the
usecase function name:

```rust
// usecase function: reserve_avatar
pub struct ReserveAvatarParams { ... }
pub struct ReserveAvatarReply { ... }

// usecase function: update_info
pub struct UserInfoUpdateParams { ... }
```

---

## 8. i18n Layer

### 8.1 Every new key goes in BOTH locale files

- `locales/en-US/main.ftl`
- `locales/zh-CN/main.ftl`

Keys use `kebab-case`, prefixed with `error-`.

---

## 9. API Handler Layer (`src/api/http/handler/`)

### 9.1 Use the usecase function via UFCS module path

```rust
use crate::usecase;

pub async fn get_info(...) -> HttpResult<UserBase> {
    let base = usecase::user::get_info(&harn, &user_token.user_id).await?;
    Ok(HttpResponse::from(base))
}
```

### 9.2 Handler names match usecase function names

`update_info` usecase → `update_info` handler.  `reserve_avatar` usecase →
`reserve_avatar` handler.

---

## 10. Router Layer (`src/api/http/router.rs`)

### 10.1 Route handler references match handler names

```rust
.route("/api/v1/user", put(user::update_info))
.route("/api/v1/user/avatar/reserve", post(user::reserve_avatar))
```

---

## 11. Harness Layer (`src/harness.rs`)

### 11.1 TestHarness must forward every trait the usecase bounds need

When a usecase uses `H: Query + ImagePut + Send + Sync`, the `TestHarness`
must implement `Query` and `ImagePut`:

```rust
use crate::domain::query::system_mail::SystemMailQueryForward;
use crate::domain::query::team::TeamQueryForward;
use crate::domain::query::user::UserQueryForward;
use crate::domain::external::image_pool::{ImageGet, ImagePut};

#[derive(Clone, Default, ForwardRefs)]
pub struct TestHarness {
    #[forward_ref(target = MemoryMockQuery, Transactional, UserQuery, TeamQuery, SystemMailQuery)]
    query: Arc<MemoryMockQuery>,
    ...
}

// Manual impls for external traits that MemoryMockQuery doesn't provide:
#[async_trait]
impl ImageGet for TestHarness {
    async fn get_signed(&self, key: &str) -> DomainResult<Url> { ... }
}

#[async_trait]
impl ImagePut for TestHarness {
    async fn put_signed(&self, key: &str) -> DomainResult<Url> { ... }
}
```

---

## 12. Full Implementation Sequence

1. **Domain model** — add new aggregate structs to `src/domain/model/aggr/`.  Apply rules §1.1–1.3.
2. **Domain query trait** — add methods to `UserQuery` or `UserQueryTransactional`.  Apply rule §2.1.
3. **Infra query entity** — add `Row` / `Entry` / `Aspect` structs.  Apply rule §3.1.
4. **Infra query impl** — implement free functions + trait impls.  Apply rules §4.1–4.2.
5. **Memory mock** — implement on `MemoryMockQuery` / `MemoryMockQueryTransactional`.  Apply rules §5.1–5.2.  Add tests.
6. **Use case data objects** — add params/reply structs.  Apply rule §7.1.
7. **Use case** — implement the function.  Apply rules §6.1–6.4.
8. **i18n keys** — add to both locale files.  Apply rule §8.1.
9. **API handler** — create/update handlers.  Apply rule §9.1–9.2.
10. **Router** — register routes.  Apply rule §10.1.
11. **TestHarness** — forward new traits.  Apply rule §11.1.
12. **Use case tests** — add tests in the usecase file (same module).
13. **`cargo check` + `cargo test`** — verify.

---

## Quick Checklist

- [ ] **All layers**: no abbreviations in new identifiers (§1.1, §7.1)
- [ ] **Domain model**: name precisely scopes what is affected (§1.2)
- [ ] **Domain query trait**: methods on correct trait — `UserQuery` vs `UserQueryTransactional` (§2.1)
- [ ] **Infra entity**: suffix matches Diesel derive — `Entry`/`Row`/`Aspect` (§3.1)
- [ ] **Infra query**: impl on correct struct — `RdbQuery` vs `RdbQueryTransactional` (§4.1)
- [ ] **Memory mock**: impl on correct struct; tests use `.boxed()` (§5.1–5.2)
- [ ] **Usecase**: UFCS for ALL trait calls (§6.1)
- [ ] **Usecase**: composite traits in where bounds (§6.2, §2.2)
- [ ] **Usecase**: `.boxed()` not `Box::pin` (§6.3)
- [ ] **Usecase**: current-user usecases accept `UserToken` by value, not `&str` (§6.5)
- [ ] **Usecase**: `transaction_scoped` only when cross-aggregate atomicity needed (§6.4)
- [ ] **i18n**: keys in both `en-US` and `zh-CN` (§8.1)
- [ ] **API handler**: names match usecase names (§9.2)
- [ ] **Router**: handler references match handler names (§10.1)
- [ ] **TestHarness**: forwards every trait the usecase bounds need (§11.1)
- [ ] **`cargo check`** compiles clean
- [ ] **`cargo test`** all tests pass
