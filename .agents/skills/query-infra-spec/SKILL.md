---
name: query-infra-spec
description: Infra query (Diesel) conventions: Row/Entry/Aspect entity naming, Aspect builder pattern, f_ column prefix, let-type-annotation (no turbofish), struct-only insert/select, From in entity module.
---

# Poprako-r Infra Query Conventions

## `infrastructure/query.rs` stays lean

Do **NOT** bloat `infrastructure/query.rs` with `#[async_trait]` impl blocks.
Place all trait impls in the per-entity files under `src/infrastructure/query/`.

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

## Rationale: Why three entity structs per table

Diesel's derive macros impose mutually incompatible type requirements on the
same table column across different operations:

| Entity | Operation | Field type | Diesel derive |
|--------|-----------|------------|---------------|
| `*Row` | SELECT | `String` (owned) | `Queryable`, `Selectable` |
| `*Entry` | INSERT | `&'a str` (borrowed) | `Insertable` |
| `*Aspect` | UPDATE … SET | `Option<&'a str>` (optional) | `AsChangeset` |

- `Queryable` requires owned types (`String`) because it deserializes rows
  into the struct.
- `Insertable` accepts borrowed references (`&str`) for zero-copy insertion.
- `AsChangeset` uses `Option<T>` to express "set only this column"; `None`
  fields are omitted from the generated `SET` clause.

A single struct cannot simultaneously derive `Queryable` (requires `String`)
and `Insertable` (requires `&'a str`), and neither can carry `Option<T>` for
the `AsChangeset` partial-update semantics. These three forms are therefore
**mandatory** — not a design choice.

Moreover, `*Row` and `*Entry` frequently diverge in which columns they
include:
- Columns with database-side defaults (e.g., `f_created_at` with `DEFAULT`)
  belong only in `*Row`.
- Write-only columns (e.g., `f_password_hash`) belong only in `*Entry` and
  are never selected back into memory.
- Read-only computed columns belong only in `*Row`.

When a table currently has identical fields between `*Row` and `*Entry`,
keep both structs rather than merging them — schema evolution will inevitably
cause divergence.

## Aspect struct construction: `new()` + builder

Every `*Aspect` struct that contains at least one `Option` field **MUST**
provide:

- A `pub fn new(updated_at: OffsetDateTime) -> Self` constructor that
  initializes all `Option` fields to `None`.
- One builder method per `Option` field, named after the field without the
  `f_` prefix, taking the inner (unwrapped) type and returning `Self`.

**Do:**

```rust
// entity/user.rs
#[derive(AsChangeset)]
#[diesel(table_name = schema::t_user)]
pub struct UserAspect<'a> {
    pub f_nickname: Option<&'a str>,
    pub f_qid: Option<&'a str>,
    pub f_updated_at: OffsetDateTime,
}

impl<'a> UserAspect<'a> {
    /// Creates a new changeset with all optional fields set to `None`.
    pub fn new(updated_at: OffsetDateTime) -> Self {
        Self {
            f_nickname: None,
            f_qid: None,
            f_updated_at: updated_at,
        }
    }

    pub fn nickname(mut self, val: &'a str) -> Self {
        self.f_nickname = Some(val);
        self
    }

    pub fn qid(mut self, val: &'a str) -> Self {
        self.f_qid = Some(val);
        self
    }
}
```

At every call site, **always** construct the changeset via the constructor +
builder chain. Struct literals for `*Aspect` are **FORBIDDEN** when a
constructor exists.

**Do:**

```rust
// user.rs — update_user
let changes = UserAspect::new(now).nickname(&input.nickname).qid(&input.qid);

// user.rs — prefill_avatar_key (single field)
let changes = UserAspect::new(now).avatar_key(key);

// member.rs — update_user_nickname
let changes = MemberAspect::new(now).user_nickname(nickname);
```

**Do NOT:**

```rust
// Struct literal — all optional fields must be spelled out, relevant fields
// are buried among None lines.
let changes = UserAspect {
    f_nickname: Some(&input.nickname),
    f_qid: Some(&input.qid),
    f_avatar_key: None,
    f_avatar_uploaded: None,
    f_last_active_at: None,
    f_updated_at: now,
};
```

When an Aspect has no `Option` fields (e.g., `MemberInvitationAspect` where
every field is mandatory), the builder pattern is unnecessary and the struct
literal form is acceptable.

## Struct-only insertion and querying

All `INSERT` and `SELECT` operations **MUST** use the corresponding entity
struct (`*Entry` / `*Row`) with `.values(&entry)` / `.select(TheRow::as_select())`.
Inline tuple-based `.values(( col.eq(val), ... ))` or raw
`.first::<(Type1, Type2)>(conn)` are **FORBIDDEN**.

**DO — Entry struct for INSERT:**
```rust
// entity/my_entity.rs
#[derive(Insertable)]
#[diesel(table_name = schema::t_my_entity)]
pub struct MyEntityEntry<'a> {
    pub f_id: &'a str,
    pub f_name: &'a str,
    pub f_created_at: OffsetDateTime,
}

// my_entity.rs (query logic)
let entry = MyEntityEntry {
    f_id: &form.id,
    f_name: &form.name,
    f_created_at: now,
};
diesel::insert_into(t_my_entity).values(&entry).execute(conn).await?;
```

**DO NOT — inline tuple for INSERT:**
```rust
// FORBIDDEN
diesel::insert_into(t_my_entity::table)
    .values((
        t_my_entity::f_id.eq(&form.id),
        t_my_entity::f_name.eq(&form.name),
        t_my_entity::f_created_at.eq(now),
    ))
    .execute(conn)
    .await?;
```

**DO — Row struct for SELECT:**
```rust
let row: MyEntityRow = t_my_entity
    .filter(f_id.eq(id))
    .select(MyEntityRow::as_select())
    .first(conn)
    .await?;
```

**DO NOT — raw tuple for SELECT:**
```rust
// FORBIDDEN
let (id, name, created_at) = t_my_entity
    .filter(f_id.eq(id))
    .first::<(String, String, OffsetDateTime)>(conn)
    .await?;
```

## Type annotation on `let`, never turbofish in query chains

On Diesel query chains, annotate the `let` binding instead of adding turbofish
type parameters inside the chain.

**Do:**
```rust
let row: UserRow = t_user
    .filter(f_id.eq(user_id))
    .select(UserRow::as_select())
    .first(conn)
    .await?;
```

**Do NOT:**
```rust
let row = t_user
    .filter(f_id.eq(user_id))
    .select(UserRow::as_select())
    .first::<UserRow>(conn)
    .await?;
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
pub async fn get_by_id(conn: &mut AsyncPgConnection, id: &str) -> DomainResult<UserAggr> {
    let row: UserRow = t_user
        .filter(f_id.eq(id))
        .select(UserRow::as_select())
        .first(conn)
        .await...?;
    Ok(row.into())
}

pub async fn create(conn: &mut AsyncPgConnection, form: &UserForm) -> DomainResult<UserAggr> {
    // ...
}
```

This keeps the `Query` / `QueryTransactional` impl blocks thin:
```rust
use async_trait::async_trait;

#[async_trait]
impl domain_query::user::UserQuery for Query {
    async fn get_by_id(&self, id: &str) -> DomainResult<UserAggr> {
        let mut conn = self.pool.get().await...?;
        get_by_id(&mut conn, id).await
    }
}

#[async_trait]
impl<'c> domain_query::user::UserQueryTransactional for QueryTransactional<'c> {
    async fn create(&mut self, form: UserForm) -> DomainResult<UserAggr> {
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
