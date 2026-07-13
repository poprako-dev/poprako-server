//! RDB-backed user repository — free query functions and thin trait impls.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_orchestra::{Run, Step};

use crate::complex::user::UserComplex;
use crate::part::repo::oper::user::{
    CreateUser, DeleteUser, FindUserInfo, GetUserCredential, GetUserInfo,
    GetUserInfoExcluded, ReserveUserAvatar, UpdateUser,
};
use crate::part::repo::user::UserRepo;
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::user::{
    UserAspect, UserCredentialRow, UserRow, UserRowEntry,
};
use crate::part_impl::shared::result::{diesel, expected, version};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{RegularError, RegularResult};

use crate::model::user::UserAvatarReservation;
use crate::model::user::UserCredential;
use crate::model::user::UserEntry;
use crate::model::user::UserInfo;
use crate::part_impl::repo::rdb_impl::schema::t_user::dsl::*;

impl UserRepo<RdbContext> for RdbRepo {}

// ── Free functions ──────────────────────────────────────────────────────────

/// Load a single user info by ID.
async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<UserInfo> {
    //
    let row: UserRow = t_user
        .filter(f_id.eq(id))
        .select(UserRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-user-not-found"))?;

    Ok(row.into())
}

/// Load credential information (password hash etc.) for a user by QID.
async fn get_credential_by_qid(
    conn: &mut RdbConn,
    qid: &str,
) -> RegularResult<UserCredential> {
    //
    let row: UserCredentialRow = t_user
        .filter(f_qid.eq(qid))
        .select(UserCredentialRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-user-not-found"))?;

    Ok(row.into())
}

/// Look up a user info by QID, returning None when not found.
async fn find_info_by_qid(
    conn: &mut RdbConn,
    qid: &str,
) -> RegularResult<Option<UserInfo>> {
    //
    let row: Option<UserRow> = t_user
        .filter(f_qid.eq(qid))
        .select(UserRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?;

    Ok(row.map(Into::into))
}

/// Insert a new user and return its info.
async fn create(
    conn: &mut RdbConn,
    entry: &UserEntry,
) -> RegularResult<UserInfo> {
    //
    let now = OffsetDateTime::now_utc();

    let entry = UserRowEntry {
        f_id: &entry.id,
        f_nickname: &entry.nickname,
        f_qid: &entry.qid,
        f_password_hash: &entry.password_hash,
        f_last_active_at: now,
        f_created_at: now,
        f_updated_at: now,
    };

    let row: UserRow = diesel::insert_into(t_user)
        .values(&entry)
        .returning(UserRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(row.into())
}

/// Update a user's QID and nickname.
async fn update_info(
    conn: &mut RdbConn,
    id: &str,
    qid: &str,
    nickname: &str,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = UserAspect::new(now).nickname(nickname).qid(qid);

    diesel::update(t_user.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Reserve a new avatar slot for a user: bump version, generate object key,
/// and return the reservation with previous key for cleanup.
async fn reserve_avatar(
    conn: &mut RdbConn,
    id: &str,
    file_ext: &str,
) -> RegularResult<UserAvatarReservation> {
    //
    let now = OffsetDateTime::now_utc();

    let (prev_key, raw_version): (Option<String>, i64) =
        diesel::update(t_user.filter(f_id.eq(id)))
            .set((
                f_avatar_key.eq::<Option<&str>>(None),
                f_avatar_uploaded.eq(false),
                f_avatar_version.eq(f_avatar_version + 1),
                f_updated_at.eq(now),
            ))
            .returning((f_avatar_key, f_avatar_version))
            .get_result::<(Option<String>, i64)>(conn)
            .await
            .map_err(diesel)?;

    let version = version(raw_version)?;

    let object_key = UserComplex::gen_avatar_key(id, version, file_ext);

    diesel::update(t_user.filter(f_id.eq(id)))
        .set((f_avatar_key.eq(Some(&object_key)), f_updated_at.eq(now)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(UserAvatarReservation {
        object_key,
        prev_object_key: prev_key,
        avatar_version: version,
    })
}

/// Mark a user avatar as uploaded, checking version staleness.
async fn mark_avatar_uploaded(
    conn: &mut RdbConn,
    id: &str,
    version: u32,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let affected = diesel::update(
        t_user
            .filter(f_id.eq(id))
            .filter(f_avatar_version.eq(i64::from(version))),
    )
    .set((f_avatar_uploaded.eq(true), f_updated_at.eq(now)))
    .execute(conn)
    .await
    .map_err(diesel)?;

    if affected == 0 {
        return Err(expected("error-avatar-version-mismatch"));
    }

    Ok(())
}

/// Update the last-active timestamp for a user.
async fn touch_last_active(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = UserAspect::new(now).last_active_at(now);

    diesel::update(t_user.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Load a user info by ID, locking the row for update.
async fn get_info_by_id_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<UserInfo> {
    //
    let row: UserRow = t_user
        .filter(f_id.eq(id))
        .select(UserRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-user-not-found"))?;

    Ok(row.into())
}

/// Delete a user by ID.
async fn delete(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    //
    diesel::delete(t_user.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

impl<'a> Run<GetUserInfo<'a>> for RdbRepo {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &GetUserInfo<'a>,
    ) -> Result<UserInfo, Self::Error> {
        match oper {
            GetUserInfo::Id { id } => {
                submit_query!(self.core, get_info_by_id, id)
            }
        }
    }
}

impl<'a> Run<GetUserCredential<'a>> for RdbRepo {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &GetUserCredential<'a>,
    ) -> Result<UserCredential, Self::Error> {
        match oper {
            GetUserCredential::Qid { qid } => {
                submit_query!(self.core, get_credential_by_qid, qid)
            }
        }
    }
}

impl<'a> Run<FindUserInfo<'a>> for RdbRepo {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &FindUserInfo<'a>,
    ) -> Result<Option<UserInfo>, Self::Error> {
        match oper {
            FindUserInfo::Qid { qid } => {
                submit_query!(self.core, find_info_by_qid, qid)
            }
        }
    }
}

impl<'a> Run<UpdateUser<'a>> for RdbRepo {
    type Error = RegularError;

    async fn run(&self, oper: &UpdateUser<'a>) -> RegularResult<()> {
        match oper {
            UpdateUser::TouchLastActive { id } => {
                submit_query!(self.core, touch_last_active, id)
            }

            UpdateUser::Info { id, qid, nickname } => {
                submit_query!(self.core, update_info, id, qid, nickname)
            }

            UpdateUser::MarkAvatarUploaded { id, avatar_version } => {
                submit_query!(
                    self.core,
                    mark_avatar_uploaded,
                    id,
                    *avatar_version
                )
            }
        }
    }
}

impl<'a> Step<CreateUser<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateUser<'a>,
    ) -> RegularResult<UserInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<'a> Step<FindUserInfo<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &FindUserInfo<'a>,
    ) -> RegularResult<Option<UserInfo>> {
        match oper {
            FindUserInfo::Qid { qid } => {
                find_info_by_qid(context.conn(), qid).await
            }
        }
    }
}

impl<'a> Step<UpdateUser<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateUser<'a>,
    ) -> RegularResult<()> {
        match oper {
            UpdateUser::Info { id, qid, nickname } => {
                update_info(context.conn(), id, qid, nickname).await
            }

            UpdateUser::MarkAvatarUploaded { id, avatar_version } => {
                mark_avatar_uploaded(context.conn(), id, *avatar_version).await
            }

            UpdateUser::TouchLastActive { id } => {
                touch_last_active(context.conn(), id).await
            }
        }
    }
}

impl<'a> Step<ReserveUserAvatar<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ReserveUserAvatar<'a>,
    ) -> RegularResult<UserAvatarReservation> {
        reserve_avatar(context.conn(), oper.id, oper.file_ext).await
    }
}

impl<'a> Step<GetUserInfoExcluded<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetUserInfoExcluded<'a>,
    ) -> RegularResult<UserInfo> {
        match oper {
            GetUserInfoExcluded::Id { id } => {
                get_info_by_id_excluded(context.conn(), id).await
            }
        }
    }
}

impl<'a> Step<DeleteUser<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteUser<'a>,
    ) -> RegularResult<()> {
        delete(context.conn(), oper.id).await
    }
}
#[cfg(all(test, feature = "repo"))]
mod tests;
