use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use tracing::Level;
use tracing::instrument;

use crate::domain::model::aggregate::user::{UserAggr, UserCredential, UserForm};
use crate::domain::query::user::UserQeury;
use crate::domain::query::user::UserQeuryTransactional;
use crate::domain::result::{DomainError, DomainResult};
use crate::infrastructure::query::Query;
use crate::infrastructure::query::QueryTransactional;
use crate::infrastructure::query::entity::user::UserEntry;
use crate::infrastructure::query::entity::user::UserInfo;
use crate::infrastructure::query::schema::t_user::dsl::*;
use crate::submit_query;
use crate::util::err::ErrorTrace as _;
use crate::util::i18n::trl;

#[instrument(skip(conn), level = Level::DEBUG)]
pub async fn get_by_id(conn: &mut AsyncPgConnection, id: String) -> DomainResult<UserAggr> {
    let info: UserInfo = t_user
        .filter(f_id.eq(&id))
        .select(UserInfo::as_select())
        .first(conn)
        .await
        .optional()?
        .ok_or(DomainError::expected_argument(trl("error-user-not-found")))
        .trace_debug()?;

    Ok(info.into())
}

pub async fn get_credential_by_qid(
    conn: &mut AsyncPgConnection,
    qid: String,
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
        .ok_or(DomainError::expected_argument(trl("error-user-not-found")))
        .trace_debug()?;

    Ok(UserCredential {
        qid: row.f_qid,
        password_hash: row.f_password_hash,
    })
}

pub async fn create(conn: &mut AsyncPgConnection, form: UserForm) -> DomainResult<UserAggr> {
    let now = OffsetDateTime::now_utc();

    let entry = UserEntry {
        f_id: form.id,
        f_nickname: form.nickname,
        f_qid: form.qid,
        f_password_hash: form.password_hash,
        f_last_active_at: now,
        f_created_at: now,
        f_updated_at: now,
    };

    diesel::insert_into(t_user)
        .values(&entry)
        .execute(conn)
        .await?;

    let info: UserInfo = t_user
        .filter(f_id.eq(&entry.f_id))
        .select(UserInfo::as_select())
        .first(conn)
        .await?;

    Ok(info.into())
}

// ── impls ──────────────────────────────────────────────────────────────────

#[async_trait]
impl UserQeury for Query {
    #[instrument(skip(self), level = Level::DEBUG)]
    async fn get_credentials_by_qid(&self, qid: String) -> DomainResult<UserCredential> {
        submit_query!(self.pool, get_credential_by_qid, qid)
    }

    #[instrument(skip(self), level = Level::DEBUG)]
    async fn get_by_id(&self, id: String) -> DomainResult<UserAggr> {
        submit_query!(self.pool, get_by_id, id)
    }
}

#[async_trait]
impl<'c> UserQeuryTransactional for QueryTransactional<'c> {
    async fn create(&mut self, form: UserForm) -> DomainResult<UserAggr> {
        create(self.conn, form).await
    }
}
