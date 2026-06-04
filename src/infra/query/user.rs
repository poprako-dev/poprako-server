use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use tracing::Level;
use tracing::instrument;

use crate::domain::model::aggr::user::{UserAggr, UserCredential, UserForm};
use crate::domain::query::user::UserQuery;
use crate::domain::query::user::UserQueryTransactional;
use crate::domain::result::{DomainError, DomainResult};
use crate::infra::query::RdbQuery;
use crate::infra::query::RdbQueryTransactional;
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
        f_qid: String,
        f_password_hash: String,
    }

    let row: Row = t_user
        .filter(f_qid.eq(&qid))
        .select((f_qid, f_password_hash))
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")).trace())?;

    Ok(UserCredential {
        qid: row.f_qid,
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
}

#[async_trait]
impl<'c> UserQueryTransactional for RdbQueryTransactional<'c> {
    async fn create(&mut self, form: &UserForm) -> DomainResult<UserAggr> {
        create(self.conn, form).await
    }
}
