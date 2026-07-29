//! RDB-backed user repository — free query functions and thin trait impls.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use crate::complex::user::UserComplex;
use crate::model::user::{
    UserAvatarReservation, UserCredential, UserEntry, UserInfo,
};
use crate::part::repo::oper::user::{
    CreateUser, DeleteUser, FindUserInfo, GetUserCredential, GetUserInfo,
    GetUserInfoExcluded, ReserveUserAvatar, UpdateUser,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::user::{
    UserAspect, UserCredentialRow, UserRow, UserRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_user::dsl::*;
use crate::part_impl::shared::result::{diesel, expected, next_version};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{BaseError, BaseResult, accept};

#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

// ── Free functions ──────────────────────────────────────────────────────────

/// Load a single user info by ID.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_by_id(conn: &mut RdbConn, id: &str) -> BaseResult<UserInfo> {
    //
    let row: UserRow = t_user
        .filter(f_id.eq(id))
        .select(UserRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-user-not-found"))?;

    accept(row.into())
}

/// Load credential information (password hash etc.) for a user by QID.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_credential_by_qid(
    conn: &mut RdbConn,
    qid: &str,
) -> BaseResult<UserCredential> {
    //
    let row: UserCredentialRow = t_user
        .filter(f_qid.eq(qid))
        .select(UserCredentialRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-user-not-found"))?;

    accept(row.into())
}

/// Look up a user info by QID, returning None when not found.
#[instrument(level = "info", err(Debug), skip_all)]
async fn find_info_by_qid(
    conn: &mut RdbConn,
    qid: &str,
) -> BaseResult<Option<UserInfo>> {
    //
    let row: Option<UserRow> = t_user
        .filter(f_qid.eq(qid))
        .select(UserRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?;

    accept(row.map(Into::into))
}

/// Insert a new user and return its info.
#[instrument(level = "info", err(Debug), skip_all)]
async fn create(conn: &mut RdbConn, entry: &UserEntry) -> BaseResult<UserInfo> {
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

    accept(row.into())
}

/// Update a user's QID and nickname.
#[instrument(level = "info", err(Debug), skip_all)]
async fn update_info(
    conn: &mut RdbConn,
    id: &str,
    qid: &str,
    nickname: &str,
) -> BaseResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = UserAspect::new(now).nickname(nickname).qid(qid);

    diesel::update(t_user.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Replace a user's password hash.
#[instrument(level = "info", err(Debug), skip_all)]
async fn update_password_hash(
    conn: &mut RdbConn,
    id: &str,
    password_hash: &str,
) -> BaseResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    diesel::update(t_user.filter(f_id.eq(id)))
        .set((f_password_hash.eq(password_hash), f_updated_at.eq(now)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Reserve a new avatar slot for a user: bump version, generate object key,
/// and return the reservation with previous key for cleanup.
#[instrument(level = "info", err(Debug), skip_all)]
async fn reserve_avatar(
    conn: &mut RdbConn,
    id: &str,
    file_ext: &str,
) -> BaseResult<UserAvatarReservation> {
    //
    let now = OffsetDateTime::now_utc();

    let (prev_key, raw_version): (Option<String>, i64) = t_user
        .filter(f_id.eq(id))
        .select((f_avatar_key, f_avatar_version))
        .for_update()
        .get_result(conn)
        .await
        .map_err(diesel)?;

    let version = next_version(raw_version)?;

    let object_key = UserComplex::gen_avatar_key(id, version, file_ext);

    diesel::update(t_user.filter(f_id.eq(id)))
        .set((
            f_avatar_key.eq(Some(&object_key)),
            f_avatar_uploaded.eq(false),
            f_avatar_version.eq(i64::from(version)),
            f_updated_at.eq(now),
        ))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(UserAvatarReservation {
        object_key,
        prev_object_key: prev_key,
        avatar_version: version,
    })
}

/// Mark a user avatar as uploaded, checking version staleness.
#[instrument(level = "info", err(Debug), skip_all)]
async fn mark_avatar_uploaded(
    conn: &mut RdbConn,
    id: &str,
    version: u32,
    avatar_key: Option<&str>,
) -> BaseResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let affected = match avatar_key {
        //
        Some(avatar_key) => {
            diesel::update(
                t_user
                    .filter(f_id.eq(id))
                    .filter(f_avatar_version.eq(i64::from(version)))
                    .filter(f_avatar_key.eq(avatar_key)),
            )
            .set((f_avatar_uploaded.eq(true), f_updated_at.eq(now)))
            .execute(conn)
            .await
        }

        None => {
            diesel::update(
                t_user
                    .filter(f_id.eq(id))
                    .filter(f_avatar_version.eq(i64::from(version))),
            )
            .set((f_avatar_uploaded.eq(true), f_updated_at.eq(now)))
            .execute(conn)
            .await
        }
    }
    .map_err(diesel)?;

    if affected == 0 {
        return Err(expected("error-avatar-version-mismatch"));
    }

    accept(())
}

/// Update the last-active timestamp for a user.
#[instrument(level = "info", err(Debug), skip_all)]
async fn touch_last_active(conn: &mut RdbConn, id: &str) -> BaseResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = UserAspect::new(now).last_active_at(now);

    diesel::update(t_user.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Load a user info by ID, locking the row for update.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_by_id_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseResult<UserInfo> {
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

    accept(row.into())
}

/// Delete a user by ID.
#[instrument(level = "info", err(Debug), skip_all)]
async fn delete(conn: &mut RdbConn, id: &str) -> BaseResult<()> {
    //
    diesel::delete(t_user.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

impl Run<GetUserInfo<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetUserInfo<'_>,
    ) -> Result<UserInfo, Self::Error> {
        match oper {
            GetUserInfo::Id { id } => {
                submit_query!(self.core, get_info_by_id, id)
            }
        }
    }
}

impl Run<GetUserCredential<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetUserCredential<'_>,
    ) -> Result<UserCredential, Self::Error> {
        match oper {
            GetUserCredential::Qid { qid } => {
                submit_query!(self.core, get_credential_by_qid, qid)
            }
        }
    }
}

impl Run<FindUserInfo<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &FindUserInfo<'_>,
    ) -> Result<Option<UserInfo>, Self::Error> {
        match oper {
            FindUserInfo::Qid { qid } => {
                submit_query!(self.core, find_info_by_qid, qid)
            }
        }
    }
}

impl Run<UpdateUser<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &UpdateUser<'_>) -> BaseResult<()> {
        match oper {
            //
            UpdateUser::TouchLastActive { id } => {
                submit_query!(self.core, touch_last_active, id)
            }

            UpdateUser::Info { id, qid, nickname } => {
                submit_query!(self.core, update_info, id, qid, nickname)
            }

            UpdateUser::MarkAvatarUploaded {
                id,
                avatar_version,
                avatar_key,
            } => {
                submit_query!(
                    self.core,
                    mark_avatar_uploaded,
                    id,
                    *avatar_version,
                    *avatar_key
                )
            }

            UpdateUser::PasswordHash { id, password_hash } => {
                submit_query!(
                    self.core,
                    update_password_hash,
                    id,
                    password_hash
                )
            }
        }
    }
}

impl Step<CreateUser<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateUser<'_>,
    ) -> BaseResult<UserInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl Step<FindUserInfo<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &FindUserInfo<'_>,
    ) -> BaseResult<Option<UserInfo>> {
        match oper {
            FindUserInfo::Qid { qid } => {
                find_info_by_qid(context.conn(), qid).await
            }
        }
    }
}

impl Step<UpdateUser<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateUser<'_>,
    ) -> BaseResult<()> {
        match oper {
            //
            UpdateUser::Info { id, qid, nickname } => {
                update_info(context.conn(), id, qid, nickname).await
            }

            UpdateUser::MarkAvatarUploaded {
                id,
                avatar_version,
                avatar_key,
            } => {
                mark_avatar_uploaded(
                    context.conn(),
                    id,
                    *avatar_version,
                    *avatar_key,
                )
                .await
            }

            UpdateUser::TouchLastActive { id } => {
                touch_last_active(context.conn(), id).await
            }

            UpdateUser::PasswordHash { id, password_hash } => {
                update_password_hash(context.conn(), id, password_hash).await
            }
        }
    }
}

impl Step<ReserveUserAvatar<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ReserveUserAvatar<'_>,
    ) -> BaseResult<UserAvatarReservation> {
        reserve_avatar(context.conn(), oper.id, oper.file_ext).await
    }
}

impl Step<GetUserInfoExcluded<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetUserInfoExcluded<'_>,
    ) -> BaseResult<UserInfo> {
        match oper {
            GetUserInfoExcluded::Id { id } => {
                get_info_by_id_excluded(context.conn(), id).await
            }
        }
    }
}

impl Step<DeleteUser<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteUser<'_>,
    ) -> BaseResult<()> {
        delete(context.conn(), oper.id).await
    }
}
