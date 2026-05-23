---
name: poprako-conventions
description: |
  Coding conventions for the poprako-r project, specifically the infra/query
  layer (Diesel). Use whenever writing or modifying code under src/infra/query/.
---

# Poprako-r Infra Query Conventions

## `infra/query.rs` stays lean

Do **NOT** bloat `infra/query.rs` with `#[async_trait]` impl blocks.
Use marker traits in the per-entity file to inherit domain traits, then place
the actual trait impls there.

Pattern: `infra/query/user.rs` defines marker traits and their impls:
```rust
pub trait UserQuery: domain_query::user::UserQeury {}
pub trait UserQeuryMut: domain_query::user::UserQeuryMut {}

impl UserQuery for Query {}
impl<'c> UserQeuryMut for TransactionalQuery<'c> {}

#[async_trait::async_trait]
impl domain_query::user::UserQeury for Query {
    async fn get_by_id(&self, id: &str) -> QueryRetVal<User> {
        let mut conn = self.pool.get()...;
        get_by_id(&mut conn, id).await
    }
}

#[async_trait::async_trait]
impl<'c> domain_query::user::UserQeuryMut for TransactionalQuery<'c> {
    async fn get_by_id(&mut self, id: &str) -> QueryRetVal<User> {
        get_by_id(self.conn, id).await
    }
}
```

`infra/query.rs` only contains:
- Module declarations
- Error `From` conversions
- `Query` and `TransactionalQuery` structs
- `Transactional` trait impl (orchestration, not query logic)

## Entity struct naming

Every Diesel entity struct in `entity/*.rs` must use one of these standardized
suffixes. Do **NOT** invent ad-hoc names.

| Suffix | Purpose | Diesel derive |
|---------|---------|---------------|
| `*Entry` | INSERT only | `Insertable` |
| `*Info` | SELECT / read | `Queryable`, `Selectable` |
| `*Aspect` | PATCH (partial update) | `AsChangeset` |
| `*Snapshot` | SAVE / full update | `Insertable` or manual |

**Do:**
```rust
#[derive(Insertable)]
#[diesel(table_name = schema::t_user)]
pub struct UserEntry { ... }

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::t_user)]
pub struct UserInfo { ... }

#[derive(AsChangeset)]
#[diesel(table_name = schema::t_user)]
pub struct UserAspect { ... }
```

**Do NOT:**
```rust
pub struct NewUser { ... }       // no: ad-hoc name for insert
pub struct UserRow { ... }       // no: "Row" is for local conflict structs only
pub struct UserChange { ... }    // no: use Aspect
```

## `From`/`Into` impls belong next to the entity type

Do **NOT** place conversion impls in query logic files
(`infra/query/user.rs`, etc.).
Place them in the entity module adjacent to the struct they convert
from (`infra/query/entity/user.rs`).

**Do:**
```rust
// infra/query/entity/user.rs
use crate::domain::model::aggr::user::User;

impl From<UserInfo> for User {
    fn from(v: UserInfo) -> Self { ... }
}
```

**Do NOT:**
```rust
// infra/query/user.rs — wrong place
impl From<UserInfo> for User { ... }
```

## Name conflicts: use a local `Row` struct

When a column name from `dsl::*` collides with a local variable,
define a local `#[derive(Queryable)] struct Row { ... }` instead of:
- suffixing variable names (`_out`, `_val`, etc.)
- aliasing the column (`use x as y_col`)

**Do:**
```rust
#[derive(Queryable)]
struct Row {
    qid: String,
    password_hash: String,
}

let row: Row = t_user
    .filter(qid.eq(qid_val))
    .select((qid, password_hash))
    .first(conn)
    .await?;
```

**Do NOT:**
```rust
// ugly suffix
let (qid_out, hash_out) = t_user....first::<(String, String)>(conn).await?;

// pointless column alias
use t_user::dsl::password_hash as pwd_hash_col;
```

## Type annotation on `let`, never turbofish in chain

On long method chains, annotate the `let` binding rather than
sprinkling turbofish (`::<Type>`) inside the chain.

**Do:**
```rust
let info: UserInfo = t_user
    .filter(id.eq(user_id))
    .select(UserInfo::as_select())
    .first(conn)
    .await?;
```

**Do NOT:**
```rust
let info = t_user
    .filter(id.eq(user_id))
    .select(UserInfo::as_select())
    .first::<UserInfo>(conn)
    .await?;
```
