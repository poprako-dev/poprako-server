//! User profile reads, locked reads, and updates.

use diesel::prelude::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
    SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::model::read::proj::user::UserInfo;
use crate::model::write::user::UserInfoRepl;
use crate::part_impl::repo::rdb_impl::entity::user::{
    UserInfoRecord, UserPack,
};
use crate::part_impl::repo::rdb_impl::schema::t_user::dsl::{
    f_id, f_qid, t_user,
};
use crate::result::{BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;

/// Find user info by QID, returning None when absent.
#[instrument(level = "info", skip_all)]
pub async fn find_info_by_qid(
    conn: &mut RdbConn,
    qid: &str,
) -> BaseRest<Option<UserInfo>> {
    //
    let record = t_user
        .filter(f_qid.eq(qid))
        .select(UserInfoRecord::as_select())
        .get_result::<UserInfoRecord>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    record.map(TryInto::try_into).transpose()
}

/// Apply a user info replacement.
#[instrument(level = "info", skip_all)]
pub async fn update_info(
    conn: &mut RdbConn,
    repl: &UserInfoRepl,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let pack = UserPack::new(now)
        .nickname(repl.trimmed_nickname())
        .qid(repl.trimmed_qid());

    diesel::update(t_user.filter(f_id.eq(&repl.id)))
        .set(&pack)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Load user info by ID, locking the record for update.
#[instrument(level = "info", skip_all)]
pub async fn get_info_by_id_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<UserInfo> {
    get_info(conn, id, true).await
}

/// Load a single user info by ID.
#[instrument(level = "info", skip_all)]
pub async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<UserInfo> {
    get_info(conn, id, false).await
}

// Load user info with optional record locking.
async fn get_info(
    conn: &mut RdbConn,
    id: &str,
    excluded: bool,
) -> BaseRest<UserInfo> {
    //
    let query = t_user
        .filter(f_id.eq(id))
        .select(UserInfoRecord::as_select());

    let record = if excluded {
        //
        query
            .for_update()
            .get_result::<UserInfoRecord>(conn)
            .await
            .optional()
            .map_err(diesel)?
    } else {
        //
        query
            .get_result::<UserInfoRecord>(conn)
            .await
            .optional()
            .map_err(diesel)?
    };

    let operation = if excluded {
        "lock user info"
    } else {
        "get user info"
    };

    let Some(record) = record else {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-user-not-found");
            error_variant = ?ExpectedVariant::Args,
            user_id = %id,
            operation,
            "user operation rejected",
        });
    };

    record.try_into()
}
