# RdbRepo Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the first production RDB repository slice: `RdbDrive`, `RdbRepo`, shared RDB context, entities, and first-scope repo/prom steps for existing tables.

**Architecture:** `RdbDrive` and `RdbRepo` are separate `part_impl` types that share one Diesel async PostgreSQL pool. `RdbDrive` implements `Drive<RdbContext>`, while `RdbRepo` implements first-scope repo/prom traits and derives the stateless `RdbRepoTransactional` handle. Diesel entity structs are precise SQL carriers and convert to active `model` values only at the repository boundary.

**Tech Stack:** Rust 2024, Diesel async PostgreSQL, `poprako-transactional`, `async-trait`, `time`, existing PopRaKo ports and step traits.

---

## Required Context

All implementer and reviewer subagents must read these files before editing:

- Spec: `docs/superpowers/specs/2026-06-30-rdb-repo-foundation-design.md`
- Project rules: `AGENTS.md`
- General Rust rules: `.agents/skills/general-conventions/SKILL.md`
- Diesel entity/query rules: `.agents/skills/repo-infra-spec/SKILL.md`
- Error rules: `.agents/skills/error-handling-spec/SKILL.md`
- Tracing rules: `.agents/skills/tracing-usage-spec/SKILL.md`
- Format string rules: `.agents/skills/format-output-spec/SKILL.md`
- Import/path rules: `.agents/skills/rust-use-style/SKILL.md` and `.agents/skills/rust-ident-style/SKILL.md`
- Active repo root: `src/part/repo.rs`
- Active shared execute trait: `src/part/shared/execute.rs`
- Active prom port: `src/part/prom.rs`
- Active `part_impl` module root: `src/part_impl.rs`
- Generated schema: `src/infra/repo/schema.rs`
- First-scope step modules:
  - `src/part/repo/step/user.rs`
  - `src/part/repo/step/team.rs`
  - `src/part/repo/step/member.rs`
  - `src/part/repo/step/member_invitation.rs`
  - `src/part/repo/step/system_mail.rs`
  - `src/part/repo/step/workset.rs`
  - `src/part/repo/step/comic.rs`

Mandatory source-of-truth references:

- `references/poprako-s/migrations/`
- `references/poprako-s/internal/infra/repo/entity/`
- Previous Rust entity/query code via `git show ba0916c^:src/infra/repo/entity/<name>.rs.bak`
- Previous Rust query code via `git show ba0916c^:src/infra/repo/<name>.rs.bak`

Existing user work:

- `src/part_impl/repo_rdb.rs` is currently untracked. Treat it as user-authored state. Read it before editing and preserve intent; do not delete it without an explicit task requirement.

Global constraints:

- Do not use active `model` structs as schema sources.
- Do not issue redundant SQL queries.
- Use exact `Entry`, `Row`, `Aspect`, and `Save` entities.
- Use `RETURNING` for write steps that return rows.
- Use batch `eq_any` include loading.
- Do not add code for `chapter`, `page`, `unit`, `assignment`, `assignment_invitation`, `announcement`, or `comment`.
- Do not start local PostgreSQL integration tests in this plan. Leave construction points ready for them.

## File Structure

Create or modify these files:

- Modify: `src/part_impl.rs`
  - Declare production modules only. Do not put RDB construction logic here.
- Create or replace: `src/part_impl/drive_rdb.rs`
  - Own `RdbDrive` and `Drive<RdbContext>`.
- Modify: `src/part_impl/repo_rdb.rs`
  - Own `RdbRepo`, `RdbRepoTransactional`, shared-backed non-transactional `conn` helper, and module declarations.
  - Import generated Diesel schema with `#[path = "../infra/repo/schema.rs"] pub mod schema;`.
- Create: `src/part_impl/repo_rdb_shared.rs`
  - Own `RdbShared`, `RdbContext`, `RdbConn`, private pool aliases, pool construction, and shared `conn` acquisition.
- Create: `src/part_impl/repo_rdb_shared/error.rs`
  - Convert Diesel, pool, serde, and invalid stored values into `RootError`.
- Create: `src/part_impl/repo_rdb/entity.rs`
  - Provide the entity module root. Submodules are declared by the tasks that
    create those files.
- Create: `src/part_impl/repo_rdb/entity/user.rs`
- Create: `src/part_impl/repo_rdb/entity/team.rs`
- Create: `src/part_impl/repo_rdb/entity/member.rs`
- Create: `src/part_impl/repo_rdb/entity/member_invitation.rs`
- Create: `src/part_impl/repo_rdb/entity/system_mail.rs`
- Create: `src/part_impl/repo_rdb/entity/workset.rs`
- Create: `src/part_impl/repo_rdb/entity/local_message.rs`
- Create: `src/part_impl/repo_rdb/entity/comic.rs`
- Create: `src/part_impl/repo_rdb/user.rs`
- Create: `src/part_impl/repo_rdb/team.rs`
- Create: `src/part_impl/repo_rdb/member.rs`
- Create: `src/part_impl/repo_rdb/member_invitation.rs`
- Create: `src/part_impl/repo_rdb/system_mail.rs`
- Create: `src/part_impl/repo_rdb/workset.rs`
- Create: `src/part_impl/repo_rdb/local_message.rs`
- Create: `src/part_impl/repo_rdb/comic.rs`
- Modify only through `just mgr-schema`: `src/infra/repo/schema.rs`
- Modify migration files only when Task 2 finds first-scope schema mismatches.

## Sequential Subagent Execution

Run one implementation subagent at a time. After each implementation subagent:

1. Run a spec compliance reviewer subagent against that task.
2. Fix every spec issue.
3. Run a code quality reviewer subagent.
4. Fix every quality issue.
5. Mark the task complete and move to the next task.

Do not dispatch implementation subagents in parallel. These tasks share migrations,
generated schema, and RDB modules.

### Task 1: Foundation And Separate Transaction Driver

**Files:**
- Modify: `src/part_impl.rs`
- Modify: `src/part_impl/repo_rdb.rs`
- Create: `src/part_impl/drive_rdb.rs`
- Create: `src/part_impl/repo_rdb_shared.rs`
- Create: `src/part_impl/repo_rdb_shared/error.rs`
- Create: `src/part_impl/repo_rdb/entity.rs`

- [ ] **Step 1: Preserve existing untracked RDB draft**

Read:

```bash
sed -n '1,220p' src/part_impl/repo_rdb.rs
```

Expected current shape before this task: the existing small draft declares
`RdbRepo` and `RdbRepoTransactional` only.

If this file has changed, incorporate the current user-authored state instead
of overwriting it blindly.

- [ ] **Step 2: Expose production modules**

Modify `src/part_impl.rs` so production RDB modules are always available, while
test mocks remain `#[cfg(test)]`:

```rust
pub mod drive_rdb;
pub mod repo_rdb;
pub mod repo_rdb_shared;

#[cfg(test)]
pub mod auth_mock;
#[cfg(test)]
pub mod effect_mock;
#[cfg(test)]
pub mod image_mock;
#[cfg(test)]
pub mod prom_mock;
#[cfg(test)]
pub mod repo_mock;
```

- [ ] **Step 3: Create shared RDB internals**

Write `src/part_impl/repo_rdb_shared.rs` with this shape:

```rust
//! Shared Diesel-backed repository internals.

use std::sync::Arc;

use diesel::result::Error as DieselError;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::PoolError;
use diesel_async::pooled_connection::deadpool::{Object, Pool};
use serde_json::Error as SerdeJsonError;

use crate::result::RootError;

pub(super) mod error;

type RdbPool = Pool<AsyncPgConnection>;
type RdbPooledConn = Object<AsyncPgConnection>;

#[derive(Clone)]
pub struct RdbShared {
    pool: Arc<RdbPool>,
}

impl RdbShared {
    pub fn from_database_url(database_url: &str) -> Result<Self, RootError> {
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);

        let pool = Pool::builder(manager)
            .build()
            .map_err(|err| error::pool_build("RdbShared::from_database_url", err))?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    pub(super) async fn conn(&self, location: &'static str) -> Result<RdbConn, RootError> {
        let conn = self.pool.get().await.map_err(|err| pool_get(location, err))?;

        Ok(RdbConn::new(conn))
    }
}

pub(super) struct RdbConn {
    conn: RdbPooledConn,
}

impl RdbConn {
    pub(super) fn new(conn: RdbPooledConn) -> Self {
        Self { conn }
    }

    pub(super) fn conn(&mut self) -> &mut AsyncPgConnection {
        &mut self.conn
    }
}

pub struct RdbContext {
    rdb_conn: RdbConn,
}

impl RdbContext {
    pub(super) fn new(rdb_conn: RdbConn) -> Self {
        Self { rdb_conn }
    }

    pub(super) fn conn(&mut self) -> &mut AsyncPgConnection {
        self.rdb_conn.conn()
    }
}

pub(super) fn pool_get(location: &'static str, err: PoolError) -> RootError {
    error::pool_get(location, err)
}

pub(super) fn diesel(location: &'static str, err: DieselError) -> RootError {
    error::diesel(location, err)
}

pub(super) fn serde(location: &'static str, err: SerdeJsonError) -> RootError {
    error::serde(location, err)
}

pub(super) fn expected(message: &str) -> RootError {
    error::expected(message)
}

pub(super) fn invalid_stored_value(
    location: &'static str,
    value: impl std::fmt::Display,
) -> RootError {
    error::invalid_stored_value(location, value)
}
```

Rules:

- `RdbShared` and `RdbContext` are public because external wiring and usecase
  generic inference must be able to name them.
- Pool aliases and pooled conn carriers are private.
- Shared helpers exposed to sibling RDB modules use `pub(super)`.
- Use `conn` for database conn identifiers.

- [ ] **Step 4: Create RDB error helpers**

Write `src/part_impl/repo_rdb_shared/error.rs`:

```rust
//! Error conversion helpers for the Diesel-backed repository.

use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel_async::pooled_connection::deadpool::{BuildError, PoolError};
use serde_json::Error as SerdeJsonError;

use poprako_util::i18n::trl;

use crate::result::{ExpectedVariant, RootError};

pub(super) fn pool_build(location: &'static str, err: BuildError) -> RootError {
    RootError::Unrecoverable {
        message: format!("[{}] failed to build pool: {}", location, err),
    }
}

pub(super) fn pool_get(location: &'static str, err: PoolError) -> RootError {
    RootError::Unrecoverable {
        message: format!("[{}] failed to get conn: {}", location, err),
    }
}

pub(super) fn diesel(location: &'static str, err: DieselError) -> RootError {
    match err {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => RootError::Expected {
            variant: ExpectedVariant::Conflict,
            message: trl("error-already-exists"),
        },
        DieselError::NotFound => RootError::Unrecoverable {
            message: format!(
                "[{}] unexpected Diesel NotFound; use optional() and map None at call site",
                location,
            ),
        },
        err => RootError::Unrecoverable {
            message: format!("[{}] diesel error: {}", location, err),
        },
    }
}

pub(super) fn serde(location: &'static str, err: SerdeJsonError) -> RootError {
    RootError::Unrecoverable {
        message: format!("[{}] serde error: {}", location, err),
    }
}

pub(super) fn expected(message: &str) -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::ArgsInvalid,
        message: trl(message),
    }
}

pub(super) fn invalid_stored_value(
    location: &'static str,
    value: impl std::fmt::Display,
) -> RootError {
    RootError::Unrecoverable {
        message: format!("[{}] invalid stored value: {}", location, value),
    }
}
```

- [ ] **Step 4.5: Create the RDB repo root**

Write `src/part_impl/repo_rdb.rs` with this structure:

```rust
//! Diesel-backed repository adapter.

use async_trait::async_trait;

use crate::util::DeriveTransactional;

use super::repo_rdb_shared;
use super::repo_rdb_shared::RdbShared;

pub mod entity;

#[path = "../infra/repo/schema.rs"]
pub mod schema;

pub struct RdbRepo {
    shared: RdbShared,
}

impl RdbRepo {
    pub fn new(shared: RdbShared) -> Self {
        Self { shared }
    }

    pub(super) async fn conn(
        &self,
        location: &'static str,
    ) -> Result<repo_rdb_shared::RdbConn, crate::result::RootError> {
        self.shared.conn(location).await
    }
}

pub struct RdbRepoTransactional;

#[async_trait]
impl DeriveTransactional for RdbRepo {
    type Transactional = RdbRepoTransactional;

    async fn transactional(&self) -> Self::Transactional {
        RdbRepoTransactional
    }
}
```

Do not put pool construction in `src/part_impl.rs`. `part_impl.rs` only organizes
modules. Construct a shared pair from callers as:

```rust
let repo_rdb_shared = RdbShared::from_database_url(database_url)?;
let repo = RdbRepo::new(repo_rdb_shared.clone());
let drive = RdbDrive::new(repo_rdb_shared);
```

- [ ] **Step 5: Create entity module root**

Write `src/part_impl/repo_rdb/entity.rs`:

```rust
//! Diesel entity types for the RDB repository.
```

- [ ] **Step 6: Create separate RDB drive**

Write `src/part_impl/drive_rdb.rs`:

```rust
//! Diesel-backed transaction driver.

use async_trait::async_trait;
use diesel_async::{AsyncConnection, TransactionManager};
use poprako_transactional::drive::Drive;
use poprako_transactional::drive::result::Error as DriveError;
use poprako_transactional::util::AsyncFnMark;

use crate::result::RootError;

use super::repo_rdb_shared::{self, RdbContext, RdbShared};

pub struct RdbDrive {
    shared: RdbShared,
}

impl RdbDrive {
    pub fn new(shared: RdbShared) -> Self {
        Self { shared }
    }
}

#[async_trait]
impl Drive<RdbContext> for RdbDrive {
    type Error = RootError;

    async fn with_context<T, E, F>(&self, f: F) -> Result<T, DriveError<E, Self::Error>>
    where
        T: Send,
        E: Send,
        for<'c> F: AsyncFnOnce(&'c mut RdbContext) -> Result<T, E>
            + AsyncFnMark<&'c mut RdbContext, Result<T, E>, Fut: Send>
            + Send,
    {
        let conn = self
            .shared
            .conn("RdbDrive::with_context")
            .await
            .map_err(DriveError::Backend)?;

        let mut rdb_context = RdbContext::new(conn);

        <AsyncPgConnection as AsyncConnection>::TransactionManager::begin_transaction(
            rdb_context.conn(),
        )
        .await
        .map_err(|err| DriveError::Backend(repo_rdb_shared::diesel("RdbDrive::with_context begin", err)))?;

        let result = f(&mut rdb_context).await;

        match result {
            Ok(value) => {
                <AsyncPgConnection as AsyncConnection>::TransactionManager::commit_transaction(
                    rdb_context.conn(),
                )
                .await
                .map_err(|err| DriveError::Backend(repo_rdb_shared::diesel("RdbDrive::with_context commit", err)))?;

                Ok(value)
            }
            Err(err) => {
                <AsyncPgConnection as AsyncConnection>::TransactionManager::rollback_transaction(
                    rdb_context.conn(),
                )
                .await
                .map_err(|rollback_err| {
                    DriveError::Backend(repo_rdb_shared::diesel(
                        "RdbDrive::with_context rollback after advance error",
                        rollback_err,
                    ))
                })?;

                Err(DriveError::Advance(err))
            }
        }
    }
}
```

This code keeps `RdbContext` owning the pooled conn and manually drives the
transaction on that conn. Do not replace this with `impl Drive<RdbContext>
for RdbRepo`; the drive/repo separation is required by application wiring.

- [ ] **Step 7: Compile and resolve lifetime shape**

Run:

```bash
cargo check
```

Keep `RdbDrive` as the only type implementing `Drive`.

- [ ] **Step 8: Commit Task 1**

Run:

```bash
git add src/part_impl.rs src/part_impl/drive_rdb.rs src/part_impl/repo_rdb.rs src/part_impl/repo_rdb/entity.rs src/part_impl/repo_rdb_shared.rs src/part_impl/repo_rdb_shared/error.rs
git commit -m "feat: add rdb repo and drive foundation"
```

Expected: one commit containing only foundation files.

### Task 2: Existing-Table Schema Audit And Migration Batch

**Files:**
- Read: `references/poprako-s/migrations/*.up.sql`
- Read: `references/poprako-s/internal/infra/repo/entity/*.go`
- Read: previous Rust entity files through `git show ba0916c^:src/infra/repo/entity/*.rs.bak`
- Modify: `migrations/**/up.sql`
- Modify: `migrations/**/down.sql`
- Regenerate: `src/infra/repo/schema.rs`

- [ ] **Step 1: Capture local first-scope schema**

Run:

```bash
sed -n '1,260p' src/infra/repo/schema.rs
```

Record current local tables for:

```text
t_user
t_team
t_member
t_member_invitation
t_system_mail
t_workset
t_local_message
t_comic
```

- [ ] **Step 2: Capture original migration shape**

Run:

```bash
for file in references/poprako-s/migrations/*.up.sql; do
  printf '\n-- %s --\n' "$file"
  rg -n 'CREATE TABLE|ALTER TABLE|CREATE INDEX|CREATE UNIQUE INDEX|REFERENCES' "$file"
done
```

Expected: output includes original `t_user`, `t_team`, `t_member`,
`t_member_invitation`, `t_system_mail`, `t_workset`, `t_oss_message`,
and `t_comic` definitions.

- [ ] **Step 3: Capture previous Rust entity shape**

Run:

```bash
for name in user team member member_invitation system_mail workset local_message; do
  git show "ba0916c^:src/infra/repo/entity/${name}.rs.bak" > "/private/tmp/${name}.entity.rs"
done
```

Then inspect:

```bash
for file in /private/tmp/*.entity.rs; do
  printf '\n-- %s --\n' "$file"
  rg -n 'pub struct|impl From|impl TryFrom|derive|table_name|pub fn new' "$file"
done
```

- [ ] **Step 4: Decide local schema deltas**

Write a short audit note in `docs/plans/rdb-first-scope-schema-audit.md` with
this exact structure:

```markdown
# RDB First-Scope Schema Audit

## Decision

Keep local `f_`-prefixed column names and generated Diesel schema as the Rust
RDB naming source. Preserve original table semantics from `poprako-s` and
previous Rust entity files.

## First-Scope Tables

| Table | Local migration status | Action |
| --- | --- | --- |
| t_user | existing | compare local columns with user migration and entity |
| t_team | existing | compare local columns with team migration and entity |
| t_member | existing | compare local columns with member migration and entity |
| t_member_invitation | existing | compare local columns with invitation migration and entity |
| t_system_mail | existing | compare local columns with system mail migration and entity |
| t_workset | existing | compare local columns with workset migration and entity |
| t_local_message | existing | use as prom store |
| t_comic | existing | compare local columns with comic migration and entity |

## Required Migration Changes

List only concrete changes found by the audit. If no migration changes are
required for a table, write `No change` in that row.
```

This file is the authoritative handoff for entity subagents.

- [ ] **Step 5: Apply migration changes in one batch**

When the audit finds concrete first-scope schema changes, create or edit migrations.
Use project commands only:

```bash
just mgr-add rdb-first-scope-schema-fix
```

Then write the `up.sql` and `down.sql` files. Keep changes limited to first-scope
tables.

When the audit finds no first-scope migration changes, do not create a migration.

- [ ] **Step 6: Run migration and regenerate schema**

Run:

```bash
just mgr-run
just mgr-schema
```

Expected:

- `just mgr-run` completes successfully.
- `src/infra/repo/schema.rs` is regenerated.

- [ ] **Step 7: Verify schema**

Run:

```bash
rg -n 'diesel::table!|t_user|t_team|t_member|t_member_invitation|t_system_mail|t_workset|t_local_message|t_comic' src/infra/repo/schema.rs
```

Expected: first-scope tables appear with the intended `f_` columns.

- [ ] **Step 8: Commit Task 2**

Run:

```bash
git add docs/plans/rdb-first-scope-schema-audit.md migrations src/infra/repo/schema.rs
git commit -m "chore: audit first-scope rdb schema"
```

When there are no migration or schema changes, commit only the audit note.

### Task 3: Entity Modules For User, Team, And Member

**Files:**
- Create: `src/part_impl/repo_rdb/entity/user.rs`
- Create: `src/part_impl/repo_rdb/entity/team.rs`
- Create: `src/part_impl/repo_rdb/entity/member.rs`
- Read: `src/model/user.rs`
- Read: `src/model/team.rs`
- Read: `src/model/member.rs`
- Read: `src/value/role.rs`

- [ ] **Step 1: Build user entities**

Create exact SQL carriers in `entity/user.rs`:

```rust
use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::user::{UserCredential, UserInfo};
use crate::part_impl::repo_rdb::schema;

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::t_user)]
pub struct UserInfoRow {
    pub f_id: String,
    pub f_nickname: String,
    pub f_qid: String,
    pub f_is_sadmin: bool,
    pub f_avatar_key: Option<String>,
    pub f_avatar_uploaded: bool,
    pub f_avatar_version: i64,
    pub f_last_active_at: OffsetDateTime,
    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::t_user)]
pub struct UserCredentialRow {
    pub f_id: String,
    pub f_password_hash: String,
}

#[derive(Insertable)]
#[diesel(table_name = schema::t_user)]
pub struct UserEntry<'a> {
    pub f_id: &'a str,
    pub f_nickname: &'a str,
    pub f_qid: &'a str,
    pub f_password_hash: &'a str,
    pub f_last_active_at: OffsetDateTime,
    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

#[derive(AsChangeset)]
#[diesel(table_name = schema::t_user)]
pub struct UserAspect<'a> {
    pub f_nickname: Option<&'a str>,
    pub f_qid: Option<&'a str>,
    pub f_avatar_key: Option<&'a str>,
    pub f_avatar_uploaded: Option<bool>,
    pub f_avatar_version: Option<i64>,
    pub f_last_active_at: Option<OffsetDateTime>,
    pub f_updated_at: OffsetDateTime,
}

impl<'a> UserAspect<'a> {
    pub fn new(updated_at: OffsetDateTime) -> Self {
        Self {
            f_nickname: None,
            f_qid: None,
            f_avatar_key: None,
            f_avatar_uploaded: None,
            f_avatar_version: None,
            f_last_active_at: None,
            f_updated_at: updated_at,
        }
    }

    pub fn nickname(mut self, value: &'a str) -> Self {
        self.f_nickname = Some(value);
        self
    }

    pub fn qid(mut self, value: &'a str) -> Self {
        self.f_qid = Some(value);
        self
    }

    pub fn avatar_key(mut self, value: &'a str) -> Self {
        self.f_avatar_key = Some(value);
        self
    }

    pub fn avatar_uploaded(mut self, value: bool) -> Self {
        self.f_avatar_uploaded = Some(value);
        self
    }

    pub fn avatar_version(mut self, value: i64) -> Self {
        self.f_avatar_version = Some(value);
        self
    }

    pub fn last_active_at(mut self, value: OffsetDateTime) -> Self {
        self.f_last_active_at = Some(value);
        self
    }
}

impl From<UserInfoRow> for UserInfo {
    fn from(value: UserInfoRow) -> Self {
        Self {
            id: value.f_id,
            qid: value.f_qid,
            nickname: value.f_nickname,
            avatar_key: value.f_avatar_key,
            avatar_uploaded: value.f_avatar_uploaded,
            avatar_version: value.f_avatar_version,
            is_sadmin: value.f_is_sadmin,
            last_active_at: value.f_last_active_at,
            created_at: value.f_created_at,
            updated_at: value.f_updated_at,
        }
    }
}

impl From<UserCredentialRow> for UserCredential {
    fn from(value: UserCredentialRow) -> Self {
        Self {
            user_id: value.f_id,
            password_hash: value.f_password_hash,
        }
    }
}
```

When generated schema requires a field not listed here, add it only to the exact
entity that selects or writes that field.

Also add these module declarations to `src/part_impl/repo_rdb/entity.rs`:

```rust
pub mod user;
pub mod team;
pub mod member;
```

- [ ] **Step 2: Build team entities**

Create `entity/team.rs` with `TeamInfoRow`, `TeamEntry`, `TeamAspect`, and
conversion to `TeamInfo`. Keep `description` conversion explicit because active
`TeamInfo.description` is `String` while the DB may store nullable text.

- [ ] **Step 3: Build member entities**

Create `entity/member.rs` with:

- `MemberInfoRow` for full member projection.
- `MemberUserInclRow` for user include population.
- `MemberTeamInclRow` for team include population.
- `MemberEntry`.
- `MemberAspect` for nickname, last-active, and role timestamp patching.

Convert role timestamp columns into `RoleMask` by checking which timestamp
columns are non-null. Convert `RoleMask` into timestamp columns for inserts and
role updates by checking each `RoleField`.

- [ ] **Step 3.5: Declare core repo modules**

Add these module declarations to `src/part_impl/repo_rdb.rs`:

```rust
pub mod user;
pub mod team;
pub mod member;
```

- [ ] **Step 4: Check entities compile**

Run:

```bash
cargo check
```

Expected: either pass, or fail only because repo implementation modules are
empty. Fix entity field names and Diesel derives until entity modules compile.

- [ ] **Step 5: Commit Task 3**

Run:

```bash
git add src/part_impl/repo_rdb/entity/user.rs src/part_impl/repo_rdb/entity/team.rs src/part_impl/repo_rdb/entity/member.rs
git commit -m "feat: add core rdb entity types"
```

### Task 4: RDB Repo Implementations For User, Team, And Member

**Files:**
- Create: `src/part_impl/repo_rdb/user.rs`
- Create: `src/part_impl/repo_rdb/team.rs`
- Create: `src/part_impl/repo_rdb/member.rs`
- Modify if needed: `src/part_impl/repo_rdb/entity/user.rs`
- Modify if needed: `src/part_impl/repo_rdb/entity/team.rs`
- Modify if needed: `src/part_impl/repo_rdb/entity/member.rs`

- [ ] **Step 1: Implement trait markers**

In each domain file, implement the repo marker traits for `RdbRepo` and
`RdbRepoTransactional`:

```rust
impl UserRepo<RdbContext> for RdbRepo {}
impl UserRepoTransactional<RdbContext> for RdbRepoTransactional {}
```

Repeat with the matching trait names for team and member.

- [ ] **Step 2: Implement user steps**

In `user.rs`, implement every step required by `UserRepo<RdbContext>` and
`UserRepoTransactional<RdbContext>`:

- `Execute<GetInfoById>`
- `Execute<GetCredentialByQid>`
- `Execute<FindInfoByQid>`
- `Advance<Create>`
- `Advance<FindInfoByQid>`
- `Advance<UpdateInfo>`
- `Advance<ReserveAvatar>`
- `Advance<MarkAvatarUploaded>`
- `Advance<TouchLastActive>`
- `Advance<GetInfoExcluded>`
- `Advance<Delete>`

Use these query rules:

- `Create` uses `insert_into(t_user).values(&UserEntry).returning(UserInfoRow::as_returning()).get_result(...)`.
- `GetInfoById` maps absence to `error-user-not-found`.
- `FindInfoByQid` returns `Ok(None)` on absence.
- `GetCredentialByQid` selects `UserCredentialRow`, not `UserInfoRow`.
- `GetInfoExcluded` uses `for_update()`.
- `ReserveAvatar` locks first, updates via `UserAspect`, and returns `UserAvatarReservation`.
- `MarkAvatarUploaded` checks version, is idempotent when already uploaded, and updates only `avatar_uploaded`.

- [ ] **Step 3: Implement team steps**

In `team.rs`, implement:

- `Execute<Create>`
- `Execute<GetInfoById>`
- `Execute<ListInfos>`
- `Execute<UpdateInfo>`
- `Execute<MarkAvatarUploaded>`
- `Advance<ReserveAvatar>`
- `Advance<MarkAvatarUploaded>`
- `Advance<GetInfoExcluded>`
- `Advance<Delete>`
- `Advance<IncrementWorksetNextIndex>`

Use `RETURNING` for `Create` and atomic `UPDATE ... RETURNING` for
`IncrementWorksetNextIndex`.

- [ ] **Step 4: Implement member steps**

In `member.rs`, implement:

- `Execute<FindInfoByUserIdAndTeamId>`
- `Execute<ListInfos>`
- `Execute<GetInfoById>`
- `Advance<Create>`
- `Advance<UpdateUserNickname>`
- `Advance<TouchLastActive>`
- `Advance<ListInfosByUserIdExcluded>`
- `Advance<FindInfoByUserIdAndTeamId>`
- `Advance<GetInfoExcluded>`
- `Advance<UpdateRole>`
- `Advance<Delete>`

For `ListInfos`, match `MemberListSpec::User` and `MemberListSpec::Team`.
Apply pagination before include loading. User and team includes must be separate
batch `eq_any` queries and must use include-specific rows.

- [ ] **Step 5: Run targeted compile**

Run:

```bash
cargo check
```

Expected: compiles through user/team/member code. Remaining failures must be
from later empty first-scope modules if those modules are declared but not yet
implemented. Fix module declaration ordering if empty modules break compilation.

- [ ] **Step 6: Commit Task 4**

Run:

```bash
git add src/part_impl/repo_rdb/user.rs src/part_impl/repo_rdb/team.rs src/part_impl/repo_rdb/member.rs src/part_impl/repo_rdb/entity/user.rs src/part_impl/repo_rdb/entity/team.rs src/part_impl/repo_rdb/entity/member.rs
git commit -m "feat: implement core rdb repo steps"
```

### Task 5: Entity Modules For Member Invitation, System Mail, And Workset

**Files:**
- Create: `src/part_impl/repo_rdb/entity/member_invitation.rs`
- Create: `src/part_impl/repo_rdb/entity/system_mail.rs`
- Create: `src/part_impl/repo_rdb/entity/workset.rs`

- [ ] **Step 1: Build member invitation entities**

Create exact entities:

- `MemberInvitationInfoRow`
- `MemberInvitationInvitorInclRow`
- `MemberInvitationEntry`
- `MemberInvitationAspect` for role updates and pending transition

Convert raw role mask into `RoleMask` using `TryFrom<u32>` after checking the DB
integer range.

- [ ] **Step 2: Build system mail entities**

Create exact entities:

- `SystemMailInfoRow`
- `SystemMailEntry`
- `SystemMailReadAspect`

Do not select receiver user data; system mail model does not carry receiver
include data.

- [ ] **Step 3: Build workset entities**

Create exact entities:

- `WorksetInfoRow`
- `WorksetEntry`
- `WorksetAspect`

Do not create `WorksetSave` in this first slice because the current active
`WorksetStep::UpdateInfo` is patch-shaped for the RDB layer.

Add these module declarations to `src/part_impl/repo_rdb/entity.rs`:

```rust
pub mod member_invitation;
pub mod system_mail;
pub mod workset;
```

Add these module declarations to `src/part_impl/repo_rdb.rs`:

```rust
pub mod member_invitation;
pub mod system_mail;
pub mod workset;
```

- [ ] **Step 4: Compile entity modules**

Run:

```bash
cargo check
```

Expected: entity module compile failures are resolved before repo step code.

- [ ] **Step 5: Commit Task 5**

Run:

```bash
git add src/part_impl/repo_rdb/entity/member_invitation.rs src/part_impl/repo_rdb/entity/system_mail.rs src/part_impl/repo_rdb/entity/workset.rs
git commit -m "feat: add invitation mail and workset rdb entities"
```

### Task 6: RDB Repo Implementations For Member Invitation, System Mail, And Workset

**Files:**
- Create: `src/part_impl/repo_rdb/member_invitation.rs`
- Create: `src/part_impl/repo_rdb/system_mail.rs`
- Create: `src/part_impl/repo_rdb/workset.rs`

- [ ] **Step 1: Implement member invitation steps**

Implement:

- `Execute<ListInfos>`
- `Execute<GetInfoById>`
- `Advance<Create>`
- `Advance<GetInfoByCodeExcluded>`
- `Advance<MarkPendingAsUsed>`
- `Advance<GetInfoById>`
- `Advance<UpdateInfo>`
- `Advance<Delete>`

Rules:

- `GetInfoByCodeExcluded` filters pending invitations and uses `for_update()`.
- `MarkPendingAsUsed` updates only pending rows and checks affected row count.
- `ListInfos` supports pending filter and invitor include as one batch query.

- [ ] **Step 2: Implement system mail steps**

Implement:

- `Execute<Send>`
- `Execute<SendBatch>`
- `Execute<ListInfosByReceiverId>`
- `Execute<ListInfosByIds>`
- `Execute<MarkRead>`

Rules:

- `SendBatch` must be one insert statement over a vector of `SystemMailEntry`.
- `ListInfosByIds` uses `eq_any` once.
- `MarkRead` updates only `f_read`.

- [ ] **Step 3: Implement workset steps**

Implement:

- `Execute<GetInfoById>`
- `Execute<ListInfosByTeamId>`
- `Execute<UpdateInfo>`
- `Advance<ListInfosByTeamIdExcluded>`
- `Advance<GetInfoExcluded>`
- `Advance<Delete>`
- `Advance<GetInfoById>`
- `Advance<Create>`
- `Advance<IncrComicNextIndex>`
- `Advance<UpdateComicCount>`

Rules:

- `Create` uses `RETURNING WorksetInfoRow`.
- `IncrComicNextIndex` uses one atomic `UPDATE ... RETURNING`.
- `UpdateComicCount` clamps at zero in SQL or by a locked update path matching
  previous behavior; do not split into unlocked read plus update.

- [ ] **Step 4: Compile**

Run:

```bash
cargo check
```

Expected: compiles through six first-scope domains. Remaining failures must be
from local-message or comic modules only.

- [ ] **Step 5: Commit Task 6**

Run:

```bash
git add src/part_impl/repo_rdb/member_invitation.rs src/part_impl/repo_rdb/system_mail.rs src/part_impl/repo_rdb/workset.rs
git commit -m "feat: implement invitation mail and workset rdb steps"
```

### Task 7: Prom Local Message Entity And Implementation

**Files:**
- Create: `src/part_impl/repo_rdb/entity/local_message.rs`
- Create: `src/part_impl/repo_rdb/local_message.rs`

- [ ] **Step 1: Build local-message entity**

Create:

- `LocalMessageEntry`
- No `LocalMessageRow`
- No `LocalMessageAspect`

For `PromStep::append`, only `LocalMessageEntry` is required.

Add this module declaration to `src/part_impl/repo_rdb/entity.rs`:

```rust
pub mod local_message;
```

Add this module declaration to `src/part_impl/repo_rdb.rs`:

```rust
pub mod local_message;
```

- [ ] **Step 2: Implement prom traits**

In `local_message.rs`, implement:

```rust
impl Prom<RdbContext> for RdbRepo {}
impl PromTransactional<RdbContext> for RdbRepoTransactional {}
```

Then implement:

```rust
impl<'a> Advance<Append<'a>, RdbContext> for RdbRepoTransactional
```

Rules:

- Serialize `Payload` to `serde_json::Value`.
- Insert one local-message row inside the transaction context.
- Use pending status matching the existing local-message schema.
- Do not build a background worker in this task.

- [ ] **Step 3: Compile**

Run:

```bash
cargo check
```

Expected: prom implementation compiles. Remaining failures must be comic-only.

- [ ] **Step 4: Commit Task 7**

Run:

```bash
git add src/part_impl/repo_rdb/entity/local_message.rs src/part_impl/repo_rdb/local_message.rs
git commit -m "feat: store prom records in rdb"
```

### Task 8: Comic Entity And Implementation

**Files:**
- Create: `src/part_impl/repo_rdb/entity/comic.rs`
- Create: `src/part_impl/repo_rdb/comic.rs`

- [ ] **Step 1: Build comic entities**

Create:

- `ComicInfoRow`
- `ComicWorksetInclRow`
- `ComicTeamInclRow`
- `ComicCreatorInclRow`
- `ComicEntry`
- `ComicAspect`

Keep include rows precise. Do not use a full `UserInfoRow`, `TeamInfoRow`, or
`WorksetInfoRow` for include paths unless every selected column is used.

Add this module declaration to `src/part_impl/repo_rdb/entity.rs`:

```rust
pub mod comic;
```

Add this module declaration to `src/part_impl/repo_rdb.rs`:

```rust
pub mod comic;
```

- [ ] **Step 2: Implement comic trait markers**

Implement:

```rust
impl ComicRepo<RdbContext> for RdbRepo {}
impl ComicRepoTransactional<RdbContext> for RdbRepoTransactional {}
```

- [ ] **Step 3: Implement non-transactional comic steps**

Implement:

- `Execute<GetInfoById>`
- `Execute<ListInfos>`
- `Execute<UpdateInfo>`
- `Execute<MarkCoverUploaded>`

Rules:

- `ListInfos` filters by `workset_id`, optional fuzzy title, and optional
  completion state.
- `ListInfos` orders by the same expression as the previous query implementation.
  When the previous implementation is missing, use `f_last_active_at.desc()`.
- Includes use one batch `eq_any` query per include option.

- [ ] **Step 4: Implement transactional comic steps**

Implement:

- `Advance<Create>`
- `Advance<GetInfoById>`
- `Advance<GetInfoExcluded>`
- `Advance<ListInfosExcluded>`
- `Advance<ReserveCover>`
- `Advance<MarkCoverUploaded>`
- `Advance<Delete>`
- `Advance<MarkCompleted>`
- `Advance<IncrChapterNextIndex>`
- `Advance<UpdateChapterCount>`
- `Advance<TouchLastActive>`

Rules:

- `Create` uses `RETURNING ComicInfoRow`.
- `GetInfoExcluded` and `ListInfosExcluded` use `for_update()`.
- `ReserveCover` locks first, updates via `ComicAspect`, and returns
  `ComicCoverReservation`.
- `IncrChapterNextIndex` uses one atomic `UPDATE ... RETURNING`.
- `UpdateChapterCount` clamps at zero by SQL expression or locked write.

- [ ] **Step 5: Compile**

Run:

```bash
cargo check
```

Expected: first-scope RDB modules compile.

- [ ] **Step 6: Commit Task 8**

Run:

```bash
git add src/part_impl/repo_rdb/entity/comic.rs src/part_impl/repo_rdb/comic.rs
git commit -m "feat: implement comic rdb steps"
```

### Task 9: Final Verification And Integration-Test Readiness Review

**Files:**
- Read all files created by Tasks 1-8.
- Modify only files with concrete compile, style, or spec issues.

- [ ] **Step 1: Run formatting**

Run:

```bash
cargo fmt
```

Expected: formatting completes.

- [ ] **Step 2: Run compile check**

Run:

```bash
cargo check
```

Expected: success. On failure, fix only first-scope RDB files or module
declarations introduced by this plan.

- [ ] **Step 3: Run style checks**

Run:

```bash
just style
```

Expected: success. When Bun dependencies or external tooling are unavailable,
record the exact failure in the final task summary and run these Rust-focused
checks instead:

```bash
just style-c
just style-d
just style-e
just style-f
just style-h
just style-i
just style-m
just style-p
just style-q
```

- [ ] **Step 4: Audit forbidden scope**

Run:

```bash
git diff --name-only HEAD~8..HEAD | rg 'chapter|page|unit|assignment|announcement|comment'
```

Expected: no output, except if the string appears inside reference text or this
plan document. No production files for later-domain repo implementations should
exist.

- [ ] **Step 5: Audit redundant-query risks**

Run:

```bash
rg -n 'insert_into|returning|first\\(|get_result|load\\(|eq_any|for_update' src/part_impl/repo_rdb
```

Review manually:

- Write steps returning rows use `RETURNING`.
- `()` write steps do not follow with select.
- Include paths use `eq_any`.
- `for_update` appears only in excluded/locked steps.

- [ ] **Step 6: Audit RdbDrive separation**

Run:

```bash
rg -n 'impl Drive<RdbContext>|struct RdbDrive|struct RdbRepo|pool\\(' src/part_impl/drive_rdb.rs src/part_impl/repo_rdb.rs
```

Expected:

- `impl Drive<RdbContext> for RdbDrive` appears in `src/part_impl/drive_rdb.rs`.
- `RdbRepo` does not implement `Drive`.
- `RdbDrive` and `RdbRepo` can both be constructed from a shared pool clone.

- [ ] **Step 7: Commit final fixes**

If Task 9 changed files, run:

```bash
git add src/part_impl src/infra/repo/schema.rs migrations docs/plans/rdb-first-scope-schema-audit.md
git commit -m "fix: finish rdb repo first-scope verification"
```

If Task 9 changed no files, do not create an empty commit.

## Final Review

After all tasks pass:

- Dispatch a final spec compliance reviewer against
  `docs/superpowers/specs/2026-06-30-rdb-repo-foundation-design.md`.
- Dispatch a final code quality reviewer across all commits from this plan.
- Run `git status --short`.
- Report:
  - Final commit range.
  - Verification commands and outcomes.
  - Any checks not run and exact reason.
  - Confirmation that later-domain repo implementations remain out of scope.
