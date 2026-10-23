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
    UserCredsRecord, UserInfoRecord, UserItem, UserPack,
};
use crate::part_impl::repo::rdb_impl::schema::t_member::dsl::{
    f_user_id as member_user_id,
    f_user_last_active_at as member_user_last_active_at,
    t_member as member_table,
};
use crate::part_impl::repo::rdb_impl::schema::t_user::dsl::{
    f_id, f_password_hash, f_qid, f_updated_at, t_user,
};
use crate::result::{BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;

/// Remove a user record from persistence.
#[instrument(level = "info", skip_all)]
pub async fn delete(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    diesel::delete(t_user.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Insert a new user record from an entry.
#[instrument(level = "info", skip_all)]
pub async fn create(
    conn: &mut RdbConn,
    entry: &UserEntry,
) -> BaseRest<UserInfo> {
    //
    let now = OffsetDateTime::now_utc();

    let item = UserItem {
        f_id: &entry.id,
        f_nickname: entry.trimmed_nickname(),
        f_qid: entry.trimmed_qid(),
        f_password_hash: &entry.password_hash,
        f_last_active_at: now,
        f_created_at: now,
        f_updated_at: now,
    };

    let record = diesel::insert_into(t_user)
        .values(&item)
        .returning(UserInfoRecord::as_returning())
        .get_result::<UserInfoRecord>(conn)
        .await
        .map_err(diesel)?;

    record.try_into()
}

/// Load user credentials by QID.
#[instrument(level = "info", skip_all)]
pub async fn get_credential_by_qid(
    conn: &mut RdbConn,
    qid: &str,
) -> BaseRest<UserCredential> {
    //
    let record = t_user
        .filter(f_qid.eq(qid))
        .select(UserCredsRecord::as_select())
        .get_result::<UserCredsRecord>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(record) = record else {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-user-not-found");
            error_variant = ?ExpectedVariant::Args,
            user_qid = %qid,
            operation = "get user credential",
            "user operation rejected",
        });
    };

    accept(record.into())
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

    let pack = UserPack::new(now).last_active_at(now);

    diesel::update(t_user.filter(f_id.eq(id)))
        .set(&pack)
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
