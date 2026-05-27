use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use crate::domain::model::aggregate::user::{UserAggr, UserCredential, UserForm};
use crate::domain::query as domain_query;
use crate::domain::result::{DomainErr, DomainResl};
use crate::infrastructure::query::Query;
use crate::infrastructure::query::TransactionalQuery;
use crate::infrastructure::query::entity::user::UserEntry;
use crate::infrastructure::query::entity::user::UserInfo;
use crate::infrastructure::query::schema::t_user::dsl::*;

pub async fn get_by_id(conn: &mut AsyncPgConnection, id: &str) -> DomainResl<UserAggr> {
    let info: UserInfo = t_user
        .filter(f_id.eq(id))
        .select(UserInfo::as_select())
        .first(conn)
        .await
        .optional()?
        .ok_or(DomainErr::expected_argument("该用户不存在".to_string()))?;

    Ok(info.into())
}

pub async fn get_credential_by_qid(
    conn: &mut AsyncPgConnection,
    qid: &str,
) -> DomainResl<UserCredential> {
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
        .ok_or(DomainErr::expected_argument("该用户不存在".to_string()))?;

    Ok(UserCredential {
        qid: row.f_qid,
        password_hash: row.f_password_hash,
    })
}

pub async fn create(conn: &mut AsyncPgConnection, form: &UserForm) -> DomainResl<UserAggr> {
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

trait UserQuery: domain_query::user::UserQeury {}

trait UserQeuryMut: domain_query::user::UserQeuryMut {}

// ── impls ──────────────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl domain_query::user::UserQeury for Query {
    async fn get_by_id(&self, id: &str) -> DomainResl<UserAggr> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| DomainErr::unrecoverable(e.to_string()))?;

        get_by_id(&mut conn, id).await
    }

    async fn get_credentials_by_qid(&self, qid: &str) -> DomainResl<UserCredential> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| DomainErr::unrecoverable(e.to_string()))?;

        get_credential_by_qid(&mut conn, qid).await
    }

    async fn create(&self, form: UserForm) -> DomainResl<UserAggr> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| DomainErr::unrecoverable(e.to_string()))?;

        create(&mut conn, &form).await
    }
}

#[async_trait::async_trait]
impl<'c> domain_query::user::UserQeuryMut for TransactionalQuery<'c> {
    async fn create(&mut self, form: UserForm) -> DomainResl<UserAggr> {
        create(self.conn, &form).await
    }
}
