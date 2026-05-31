---
name: query-infra-spec
description: |
  Coding conventions for the poprako-r infra/query layer (Diesel ORM).
  Use whenever writing or modifying code under src/infrastructure/query/.
---

# Poprako-r Infra Query Conventions

## `infrastructure/query.rs` stays lean

Do **NOT** bloat `infrastructure/query.rs` with `#[async_trait]` impl blocks.
Use marker traits in the per-entity file to inherit domain traits, then place
the actual trait impls there.

Pattern: `infrastructure/query/user.rs` defines marker traits and their impls:

```rust
use async_trait::async_trait;

trait UserQuery: domain_query::user::UserQuery {}
trait UserQueryTransactional: domain_query::user::UserQueryTransactional {}

impl UserQuery for Query {}
impl<'c> UserQueryTransactional for QueryTransactional<'c> {}

#[async_trait]
impl domain_query::user::UserQuery for Query {
    async fn get_by_id(&self, id: &str) -> DomainResl<UserAggr> {
        let mut conn = self.pool.get()...;
        get_by_id(&mut conn, id).await
    }
}

#[async_trait]
impl<'c> domain_query::user::UserQueryTransactional for QueryTransactional<'c> {
    async fn create(&mut self, form: UserForm) -> DomainResl<UserAggr> {
        create(self.conn, &form).await
    }
}
```

`infrastructure/query.rs` only contains:
- Module declarations
- Error `From` conversions
- `Query` and `QueryTransactional` structs
- `Transactional` trait impl (orchestration, not query logic)

## Entity struct naming

Every Diesel entity struct in `entity/*.rs` must use one of these standardized
suffixes. Do **NOT** invent ad-hoc names.

| Suffix | Purpose | Diesel derive |
|---------|---------|---------------|
| `*Entry` | INSERT only | `Insertable` |
| `*Row` | SELECT / read | `Queryable`, `Selectable` |
| `*Aspect` | PATCH (partial update) | `AsChangeset` |
| `*Snapshot` | SAVE / full update | `Insertable` or manual |

Special-purpose read structs (e.g., credentials) may use a descriptive name
without the standard suffix (`UserCredential`).

**Do:**
```rust
use diesel::prelude::*;

#[derive(Insertable)]
#[diesel(table_name = schema::t_user)]
pub struct UserEntry { ... }

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::t_user)]
pub struct UserRow { ... }

#[derive(AsChangeset)]
#[diesel(table_name = schema::t_user)]
pub struct UserAspect { ... }
```

**Do NOT:**
```rust
pub struct NewUser { ... }       // no: ad-hoc name for insert
pub struct UserChange { ... }    // no: use Aspect
```

## Database column prefix

All database columns use the `f_` prefix (e.g., `f_id`, `f_nickname`).
Entity struct fields mirror this prefix exactly:

```rust
use diesel::prelude::*;

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::t_user)]
pub struct UserRow {
    pub f_id: String,
    pub f_nickname: String,
    pub f_qid: String,
    // ...
}
```

Domain aggregates drop the prefix in `From` conversions:
```rust
impl From<UserRow> for UserAggr {
    fn from(v: UserRow) -> Self {
        Self {
            id: v.f_id,
            nickname: v.f_nickname,
            // ...
        }
    }
}
```

## `From`/`Into` impls belong next to the entity type

Do **NOT** place conversion impls in query logic files
(`infrastructure/query/user.rs`, etc.).
Place them in the entity module adjacent to the struct they convert
from (`infrastructure/query/entity/user.rs`).

**Do:**
```rust
// infrastructure/query/entity/user.rs
use crate::domain::model::aggregate::user::UserAggr;

impl From<UserRow> for UserAggr {
    fn from(v: UserRow) -> Self { ... }
}
```

**Do NOT:**
```rust
// infrastructure/query/user.rs — wrong place
impl From<UserRow> for UserAggr { ... }
```

## Name conflicts: use a local `Row` struct

When a column name from `dsl::*` collides with a local variable,
define a local `#[derive(Queryable)] struct Row { ... }` instead of:
- suffixing variable names (`_out`, `_val`, etc.)
- aliasing the column (`use x as y_col`)

**Do:**
```rust
use diesel::prelude::*;

#[derive(Queryable)]
struct Row {
    f_qid: String,
    f_password_hash: String,
}

let row: Row = t_user
    .filter(f_qid.eq(qid_val))
    .select((f_qid, f_password_hash))
    .first(conn)
    .await?;
```

**Do NOT:**
```rust
// ugly suffix
let (qid_out, hash_out) = t_user....first::<(String, String)>(conn).await?;

// pointless column alias
use t_user::dsl::f_password_hash as pwd_hash_col;
```

## Public query functions accept `&mut AsyncPgConnection`

Per-entity query files (`infrastructure/query/user.rs`, etc.) should expose
freestanding `pub async fn` helpers that take `conn: &mut AsyncPgConnection`.
These are then called by both the `Query` and `QueryTransactional` impl blocks.

```rust
// infrastructure/query/user.rs
pub async fn get_by_id(conn: &mut AsyncPgConnection, id: &str) -> DomainResl<UserAggr> {
    let row: UserRow = t_user
        .filter(f_id.eq(id))
        .select(UserRow::as_select())
        .first(conn)
        .await...?;
    Ok(row.into())
}

pub async fn create(conn: &mut AsyncPgConnection, form: &UserForm) -> DomainResl<UserAggr> {
    // ...
}
```

This keeps the `Query` / `QueryTransactional` impl blocks thin:
```rust
use async_trait::async_trait;

#[async_trait]
impl domain_query::user::UserQuery for Query {
    async fn get_by_id(&self, id: &str) -> DomainResl<UserAggr> {
        let mut conn = self.pool.get().await...?;
        get_by_id(&mut conn, id).await
    }
}

#[async_trait]
impl<'c> domain_query::user::UserQueryTransactional for QueryTransactional<'c> {
    async fn create(&mut self, form: UserForm) -> DomainResl<UserAggr> {
        create(self.conn, &form).await
    }
}
```

## Domain `QueryTransactional` trait aggregation

The domain's `QueryTransactional` trait in `domain/query.rs` aggregates all
per-entity `*Transactional` traits via supertraits:

```rust
pub trait QueryTransactional:
    Send + UserQueryTransactional + MemberQueryTransactional + MemberInvitationQueryTransactional
{
}
```

When adding a new entity with transactional operations, add its `*Transactional` trait
as a supertrait bound here so that `QueryTransactional` remains usable in
`run_in_transaction` closures.
