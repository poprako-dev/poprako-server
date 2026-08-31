//! Account lifecycle, credentials, and activity persistence.

use diesel::prelude::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
    SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::write::user::{UserCredsRepl, UserEntry};
use crate::part_impl::repo::rdb_impl::entity::user::{
    UserAspectRow, UserCredsRow, UserEntryRow, UserInfoRow,
};
use crate::part_impl::repo::rdb_impl::schema::t_member::dsl::{
    f_user_id as member_user_id,
    f_user_last_active_at as member_user_last_active_at,
    t_member as member_table,
};
use crate::part_impl::repo::rdb_impl::schema::t_user::dsl::{
    f_id, f_password_hash, f_qid, f_updated_at, t_user,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;

/// Remove a user row from persistence.
#[instrument(level = "info", skip_all)]
pub async fn delete(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    diesel::delete(t_user.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Insert a new user row from an entry.
#[instrument(level = "info", skip_all)]
pub async fn create(
    conn: &mut RdbConn,
    entry: &UserEntry,
) -> BaseRest<UserInfo> {
    //
    let now = OffsetDateTime::now_utc();

    let entry = UserEntryRow {
        f_id: &entry.id,
        f_nickname: &entry.nickname,
        f_qid: &entry.qid,
        f_password_hash: &entry.password_hash,
        f_last_active_at: now,
        f_created_at: now,
        f_updated_at: now,
    };

    let row = diesel::insert_into(t_user)
        .values(&entry)
        .returning(UserInfoRow::as_returning())
        .get_result::<UserInfoRow>(conn)
        .await
        .map_err(diesel)?;

    row.try_into()
}

/// Load user credentials by QID.
#[instrument(level = "info", skip_all)]
pub async fn get_credential_by_qid(
    conn: &mut RdbConn,
    qid: &str,
) -> BaseRest<UserCredential> {
    //
    let row = t_user
        .filter(f_qid.eq(qid))
        .select(UserCredsRow::as_select())
        .get_result::<UserCredsRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
        //
        let message = trl("error-user-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            user_qid = %qid,
            operation = "get user credential",
            "expected user error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    };

    accept(row.into())
}

/// Update the user password hash.
#[instrument(level = "info", skip_all)]
pub async fn update_password_hash(
    conn: &mut RdbConn,
    repl: &UserCredsRepl,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    diesel::update(t_user.filter(f_id.eq(&repl.id)))
        .set((
            f_password_hash.eq(&repl.password_hash),
            f_updated_at.eq(now),
        ))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Update user and membership activity timestamps.
#[instrument(level = "info", skip_all)]
pub async fn touch_last_active(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = UserAspectRow::new(now).last_active_at(now);

    diesel::update(t_user.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    diesel::update(member_table.filter(member_user_id.eq(id)))
        .set(member_user_last_active_at.eq(now))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}
