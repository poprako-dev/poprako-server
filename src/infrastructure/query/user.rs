use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use tracing::Level;
use tracing::instrument;

use crate::domain::model::aggregate::user::{UserAggr, UserCredential, UserForm};
use crate::domain::query as domain_query;
use crate::domain::result::{DomainError, DomainResult};
use crate::infrastructure::query::Query;
use crate::infrastructure::query::TransactionalQuery;
use crate::infrastructure::query::entity::user::UserEntry;
use crate::infrastructure::query::entity::user::UserInfo;
use crate::infrastructure::query::schema::t_user::dsl::*;
use crate::util::err::ErrorTrace as _;
use crate::util::i18n::trl;

#[instrument(skip(conn), level = Level::DEBUG)]
pub async fn get_by_id(conn: &mut AsyncPgConnection, id: &str) -> DomainResult<UserAggr> {
    let info: UserInfo = t_user
        .filter(f_id.eq(id))
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
    qid: &str,
) -> DomainResult<UserCredential> {
    #[derive(Queryable)]
    struct Row {
        f_qid: String,
        f_password_hash: String,
    }

    let row: Row = t_user
        .filter(f_qid.eq(qid))
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

pub async fn create(conn: &mut AsyncPgConnection, form: &UserForm) -> DomainResult<UserAggr> {
    let now = OffsetDateTime::now_utc();

    let entry = UserEntry {
        f_id: form.id.clone(),
        f_nickname: form.nickname.clone(),
        f_qid: form.qid.clone(),
        f_password_hash: form.password_hash.clone(),
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

// ── Marker traits ──────────────────────────────────────────────────────────

/// Blanket-impl marker: every [`Query`] is a [`UserQeury`](crate::domain::query::user::UserQeury).
/// Exists only so that `TransactionalQuery` bounds can reference a single trait instead of
/// listing every super-trait on every impl block.
trait UserQuery: domain_query::user::UserQeury {}

/// Blanket-impl marker: every [`TransactionalQuery`] is a
/// [`UserQeuryMut`](crate::domain::query::user::UserQeuryMut).
trait UserQeuryMut: domain_query::user::UserQeuryMut {}

// ── impls ──────────────────────────────────────────────────────────────────

#[async_trait]
impl domain_query::user::UserQeury for Query {
    #[instrument(skip(self), level = Level::DEBUG)]
    async fn get_by_id(&self, id: &str) -> DomainResult<UserAggr> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| {
                DomainError::unrecoverable(format!(
                    "[Query::get_by_id] error getting connection: {}",
                    e
                ))
            })
            .trace_error()?;

        get_by_id(&mut conn, id).await
    }

    #[instrument(skip(self), level = Level::DEBUG)]
    async fn get_credentials_by_qid(&self, qid: &str) -> DomainResult<UserCredential> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| {
                DomainError::unrecoverable(format!(
                    "[Query::get_credentials_by_qid] error getting connection: {}",
                    e
                ))
            })
            .trace_error()?;

        get_credential_by_qid(&mut conn, qid).await
    }

    #[instrument(skip(self, form), level = Level::DEBUG)]
    async fn create(&self, form: UserForm) -> DomainResult<UserAggr> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| {
                DomainError::unrecoverable(format!(
                    "[Query::create] error getting connection: {}",
                    e
                ))
            })
            .trace_error()?;

        create(&mut conn, &form).await
    }
}

#[async_trait]
impl<'c> domain_query::user::UserQeuryMut for TransactionalQuery<'c> {
    async fn create(&mut self, form: UserForm) -> DomainResult<UserAggr> {
        create(self.conn, &form).await
    }
}
