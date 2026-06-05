use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use tracing::Level;
use tracing::instrument;

use crate::domain::model::aggr::user::{UserAggr, UserCredential, UserForm, UserInfoUpdate};
use crate::domain::query::user::UserQuery;
use crate::domain::query::user::UserQueryTransactional;
use crate::domain::result::{DomainError, DomainResult};
use crate::infra::query::RdbQuery;
use crate::infra::query::RdbQueryTransactional;
use crate::infra::query::entity::user::UserAspect;
use crate::infra::query::entity::user::UserEntry;
use crate::infra::query::entity::user::UserRow;
use crate::infra::query::schema::t_user::dsl::*;
use crate::submit_query;
use poprako_util::i18n::trl;

#[instrument(skip(conn), level = Level::DEBUG)]
pub async fn get_by_id(conn: &mut AsyncPgConnection, id: &str) -> DomainResult<UserAggr> {
    let row: UserRow = t_user
        .filter(f_id.eq(&id))
        .select(UserRow::as_select())
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")).trace())?;

    Ok(row.into())
}

pub async fn get_credential_by_qid(
    conn: &mut AsyncPgConnection,
    qid: &str,
) -> DomainResult<UserCredential> {
    #[derive(Queryable)]
    struct Row {
        f_id: String,
        f_password_hash: String,
    }

    let row: Row = t_user
        .filter(f_qid.eq(&qid))
        .select((f_id, f_password_hash))
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")).trace())?;

    Ok(UserCredential {
        user_id: row.f_id,
        password_hash: row.f_password_hash,
    })
}

pub async fn create(conn: &mut AsyncPgConnection, form: &UserForm) -> DomainResult<UserAggr> {
    let now = OffsetDateTime::now_utc();

    let entry = UserEntry {
        f_id: &form.id,
        f_nickname: &form.nickname,
        f_qid: &form.qid,
        f_password_hash: &form.password_hash,
        f_last_active_at: now,
        f_created_at: now,
        f_updated_at: now,
    };

    diesel::insert_into(t_user)
        .values(&entry)
        .execute(conn)
        .await?;

    let row: UserRow = t_user
        .filter(f_id.eq(&entry.f_id))
        .select(UserRow::as_select())
        .first(conn)
        .await?;

    Ok(row.into())
}

pub async fn update_user(
    conn: &mut AsyncPgConnection,
    input: &UserInfoUpdate,
) -> DomainResult<UserAggr> {
    let now = OffsetDateTime::now_utc();

    let changes = UserAspect::new(now)
        .nickname(&input.nickname)
        .qid(&input.qid);

    let affected = diesel::update(t_user.filter(f_id.eq(&input.id)))
        .set(&changes)
        .execute(conn)
        .await?;

    if affected == 0 {
        return Err(DomainError::expected_argument(trl("error-user-not-found")).trace());
    }

    let row: UserRow = t_user
        .filter(f_id.eq(&input.id))
        .select(UserRow::as_select())
        .first(conn)
        .await?;

    Ok(row.into())
}

pub async fn prefill_avatar_key(
    conn: &mut AsyncPgConnection,
    id: &str,
    key: &str,
) -> DomainResult<()> {
    let now = OffsetDateTime::now_utc();

    let changes = UserAspect::new(now).avatar_key(key);

    let affected = diesel::update(t_user.filter(f_id.eq(id)))
        .set(&changes)
        .execute(conn)
        .await?;

    if affected == 0 {
        return Err(DomainError::expected_argument(trl("error-user-not-found")).trace());
    }

    Ok(())
}

pub async fn mark_avatar_uploaded(conn: &mut AsyncPgConnection, id: &str) -> DomainResult<()> {
    let now = OffsetDateTime::now_utc();

    let changes = UserAspect::new(now).avatar_uploaded(true);

    let affected = diesel::update(t_user.filter(f_id.eq(id)))
        .set(&changes)
        .execute(conn)
        .await?;

    if affected == 0 {
        return Err(DomainError::expected_argument(trl("error-user-not-found")).trace());
    }

    Ok(())
}

pub async fn touch_last_active(conn: &mut AsyncPgConnection, id: &str) -> DomainResult<()> {
    let now = OffsetDateTime::now_utc();

    // Lock the user row to serialise concurrent touch/creation operations
    // for this user.  This prevents phantom reads on the member table:
    // any concurrent member INSERT that references the user must also
    // acquire this row lock first.
    let exists = t_user
        .filter(f_id.eq(id))
        .select(f_id)
        .for_update()
        .first::<String>(conn)
        .await
        .optional()?;

    if exists.is_none() {
        return Err(DomainError::expected_argument(trl("error-user-not-found")).trace());
    }

    let changes = UserAspect::new(now).last_active_at(now);

    diesel::update(t_user.filter(f_id.eq(id)))
        .set(&changes)
        .execute(conn)
        .await?;

    Ok(())
}

// ── impls ──────────────────────────────────────────────────────────────────

#[async_trait]
impl UserQuery for RdbQuery {
    #[instrument(skip(self), level = Level::DEBUG)]
    async fn get_credentials_by_qid(&self, qid: &str) -> DomainResult<UserCredential> {
        submit_query!(self.pool, get_credential_by_qid, qid)
    }

    #[instrument(skip(self), level = Level::DEBUG)]
    async fn get_by_id(&self, id: &str) -> DomainResult<UserAggr> {
        submit_query!(self.pool, get_by_id, id)
    }

    #[instrument(skip(self), level = Level::DEBUG)]
    async fn prefill_avatar_key(&self, id: &str, key: &str) -> DomainResult<()> {
        submit_query!(self.pool, prefill_avatar_key, id, key)
    }

    #[instrument(skip(self), level = Level::DEBUG)]
    async fn mark_avatar_uploaded(&self, id: &str) -> DomainResult<()> {
        submit_query!(self.pool, mark_avatar_uploaded, id)
    }

}

#[async_trait]
impl<'c> UserQueryTransactional for RdbQueryTransactional<'c> {
    async fn create(&mut self, form: &UserForm) -> DomainResult<UserAggr> {
        create(self.conn, form).await
    }

    async fn update_info(&mut self, input: &UserInfoUpdate) -> DomainResult<UserAggr> {
        update_user(self.conn, input).await
    }

    #[instrument(skip(self), level = Level::DEBUG)]
    async fn touch_last_active(&mut self, id: &str) -> DomainResult<()> {
        touch_last_active(self.conn, id).await
    }
}
