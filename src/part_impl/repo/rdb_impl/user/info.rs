//! User profile reads, locked reads, and updates.

use diesel::prelude::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
    SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::model::read::proj::user::UserInfo;
use crate::model::write::user::UserInfoRepl;
use crate::part_impl::repo::rdb_impl::entity::user::{
    UserAspectRow, UserInfoRow,
};
use crate::part_impl::repo::rdb_impl::schema::t_user::dsl::{
    f_id, f_qid, t_user,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::RdbConn;
use crate::shared::result::diesel;

#[instrument(level = "info", skip_all)]
/// Find user info by QID, returning None when absent.
pub async fn find_info_by_qid(
    conn: &mut RdbConn,
    qid: &str,
) -> BaseRest<Option<UserInfo>> {
    //
    let row = t_user
        .filter(f_qid.eq(qid))
        .select(UserInfoRow::as_select())
        .get_result::<UserInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    row.map(TryInto::try_into).transpose()
}

#[instrument(level = "info", skip_all)]
/// Apply a user info replacement.
pub async fn update_info(
    conn: &mut RdbConn,
    repl: &UserInfoRepl,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = UserAspectRow::new(now)
        .nickname(&repl.nickname)
        .qid(&repl.qid);

    diesel::update(t_user.filter(f_id.eq(&repl.id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

#[instrument(level = "info", skip_all)]
/// Load user info by ID, locking the row for update.
pub async fn get_info_by_id_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<UserInfo> {
    get_info(conn, id, true).await
}

#[instrument(level = "info", skip_all)]
/// Load a single user info by ID.
pub async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<UserInfo> {
    get_info(conn, id, false).await
}

// Load user info with optional row locking.
async fn get_info(
    conn: &mut RdbConn,
    id: &str,
    excluded: bool,
) -> BaseRest<UserInfo> {
    //
    let query = t_user.filter(f_id.eq(id)).select(UserInfoRow::as_select());

    let row = match excluded {
        //
        true => query
            .for_update()
            .get_result::<UserInfoRow>(conn)
            .await
            .optional()
            .map_err(diesel)?,

        false => query
            .get_result::<UserInfoRow>(conn)
            .await
            .optional()
            .map_err(diesel)?,
    };

    let operation = match excluded {
        //
        true => "lock user info",

        false => "get user info",
    };

    let Some(row) = row else {
        //
        let message = trl("error-user-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            user_id = %id,
            operation,
            "expected user error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    };

    row.try_into()
}
