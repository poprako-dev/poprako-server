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
use crate::value::image::{ImageExt, ImageHash};

/// User RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

// ── Free functions ──────────────────────────────────────────────────────────

// Remove a user row from persistence.
#[instrument(level = "info", err(Debug), skip_all)]
async fn delete(conn: &mut RdbConn, id: &str) -> BaseResult<()> {
    //
    // Execute hard delete and map DB errors to repository error type.
    diesel::delete(t_user.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Load credential material for authentication operations from the same backing row.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_credential_by_qid(
    conn: &mut RdbConn,
    qid: &str,
) -> BaseResult<UserCredential> {
    //
    // Query only credential columns and convert them to user credential DTO.
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

// Return user info by QID, yielding `None` instead of an error when missing.
#[instrument(level = "info", err(Debug), skip_all)]
async fn find_info_by_qid(
    conn: &mut RdbConn,
    qid: &str,
) -> BaseResult<Option<UserInfo>> {
    //
    // Keep lookup soft-fail to allow callers to branch on existence.
    let row: Option<UserRow> = t_user
        .filter(f_qid.eq(qid))
        .select(UserRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?;

    row.map(TryInto::try_into).transpose()
}

// Insert a new user row and return the persisted info payload.
#[instrument(level = "info", err(Debug), skip_all)]
async fn create(conn: &mut RdbConn, entry: &UserEntry) -> BaseResult<UserInfo> {
    //
    // Populate required identity and timestamp columns, then fetch created row.
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

    row.try_into()
}

// Update mutable identity fields (`qid`, `nickname`) for an existing user.
#[instrument(level = "info", err(Debug), skip_all)]
async fn update_info(
    conn: &mut RdbConn,
    id: &str,
    qid: &str,
    nickname: &str,
) -> BaseResult<()> {
    //
    // Apply one write that updates both fields and returns success when DB update succeeds.
    let now = OffsetDateTime::now_utc();

    let aspect = UserAspect::new(now).nickname(nickname).qid(qid);

    diesel::update(t_user.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Replace a user's password hash and refresh the row-level update timestamp.
#[instrument(level = "info", err(Debug), skip_all)]
async fn update_password_hash(
    conn: &mut RdbConn,
    id: &str,
    password_hash: &str,
) -> BaseResult<()> {
    //
    // Persist credential changes and bump `f_updated_at` in one SQL statement.
    let now = OffsetDateTime::now_utc();

    diesel::update(t_user.filter(f_id.eq(id)))
        .set((f_password_hash.eq(password_hash), f_updated_at.eq(now)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Reserve an avatar object version, reusing existing key when hash unchanged.
#[instrument(level = "info", err(Debug), skip_all)]
async fn reserve_avatar(
    conn: &mut RdbConn,
    id: &str,
    image_hash: &ImageHash,
    image_ext: ImageExt,
) -> BaseResult<UserAvatarReservation> {
    //
    // Lock the target row, compare hash/ext, then either reuse or advance avatar version.
    let now = OffsetDateTime::now_utc();

    let (prev_key, uploaded, raw_version, stored_hash, stored_ext): (
        Option<String>,
        bool,
        i64,
        Vec<u8>,
        String,
    ) = t_user
        .filter(f_id.eq(id))
        .select((
            f_avatar_key,
            f_avatar_uploaded,
            f_avatar_version,
            f_avatar_hash,
            f_avatar_extension,
        ))
        .for_update()
        .get_result(conn)
        .await
        .map_err(diesel)?;

    let same_hash =
        prev_key.is_some() && stored_hash.as_slice() == image_hash.as_bytes();

    if same_hash && stored_ext != image_ext.suffix() {
        return Err(expected("error-invalid-image-extension"));
    }

    if same_hash {
        //
        let object_key = prev_key.ok_or_else(|| BaseError::Unrecoverable {
            message: "[reserve_avatar] pending avatar key is missing".into(),
        })?;

        return accept(UserAvatarReservation {
            object_key,
            prev_object_key: None,
            avatar_version: u32::try_from(raw_version).map_err(|_| {
                BaseError::Unrecoverable {
                    message: "[reserve_avatar] avatar version is invalid"
                        .into(),
                }
            })?,
            upload_required: !uploaded,
        });
    }

    let version = next_version(raw_version)?;

    let object_key =
        UserComplex::gen_avatar_key(id, version, image_ext.suffix());

    diesel::update(t_user.filter(f_id.eq(id)))
        .set((
            f_avatar_key.eq(Some(&object_key)),
            f_avatar_uploaded.eq(false),
            f_avatar_version.eq(i64::from(version)),
            f_avatar_hash.eq(image_hash.as_bytes().to_vec()),
            f_avatar_extension.eq(image_ext.suffix()),
            f_updated_at.eq(now),
        ))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(UserAvatarReservation {
        object_key,
        prev_object_key: prev_key,
        avatar_version: version,
        upload_required: true,
    })
}

// Mark upload result for an avatar with optimistic-version guarding.
#[instrument(level = "info", err(Debug), skip_all)]
async fn mark_avatar_uploaded(
    conn: &mut RdbConn,
    id: &str,
    version: u32,
    avatar_key: Option<&str>,
    avatar_uploaded: bool,
) -> BaseResult<()> {
    //
    // Only write when version (and optional key) match; return mismatch error if stale.
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
            .set((f_avatar_uploaded.eq(avatar_uploaded), f_updated_at.eq(now)))
            .execute(conn)
            .await
        }

        None => {
            diesel::update(
                t_user
                    .filter(f_id.eq(id))
                    .filter(f_avatar_version.eq(i64::from(version))),
            )
            .set((f_avatar_uploaded.eq(avatar_uploaded), f_updated_at.eq(now)))
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

// Touch `last_active_at` for heartbeat and usage tracking.
#[instrument(level = "info", err(Debug), skip_all)]
async fn touch_last_active(conn: &mut RdbConn, id: &str) -> BaseResult<()> {
    //
    // Keep access timestamp current for activity-driven features.
    let now = OffsetDateTime::now_utc();

    let aspect = UserAspect::new(now).last_active_at(now);

    diesel::update(t_user.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Load one user info row with `FOR UPDATE` lock for follow-up writes.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_by_id_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseResult<UserInfo> {
    //
    // Use a row lock so later mutation in the same transaction is serialized.
    let row: UserRow = t_user
        .filter(f_id.eq(id))
        .select(UserRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-user-not-found"))?;

    row.try_into()
}

// Load one user info row by primary key and map DB row into response model.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_by_id(conn: &mut RdbConn, id: &str) -> BaseResult<UserInfo> {
    //
    // Query `t_user` by `f_id`, fail with `error-user-not-found` when absent.
    let row: UserRow = t_user
        .filter(f_id.eq(id))
        .select(UserRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-user-not-found"))?;

    row.try_into()
}

impl Run<GetUserInfo<'_>> for RdbRepo {
    // Use `BaseError` for non-transactional repository reads.
    type Error = BaseError;

    // Route read by ID into the shared `submit_query!` orchestration.
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
    // Use `BaseError` for non-transactional credential reads.
    type Error = BaseError;

    // Route credential read by QID to the shared repository query path.
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
    // Use `BaseError` for non-transactional optional reads.
    type Error = BaseError;

    // Route optional user lookup by QID to shared query layer.
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
    // Use `BaseError` for non-transactional user mutations.
    type Error = BaseError;

    // Map each update variant to a dedicated helper with explicit argument flow.
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
                avatar_uploaded,
            } => {
                submit_query!(
                    self.core,
                    mark_avatar_uploaded,
                    id,
                    *avatar_version,
                    *avatar_key,
                    *avatar_uploaded
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
    // Keep transaction-scoped operations on one repository error type.
    type Error = BaseError;

    // Insert new user rows inside provided transaction context.
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
    // Keep transaction-scoped reads on one repository error type.
    type Error = BaseError;

    // Resolve soft-miss lookup inside caller-owned transaction context.
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
    // Keep transaction-scoped updates on one repository error type.
    type Error = BaseError;

    // Dispatch each mutable user operation to one explicit DB helper.
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
                avatar_uploaded,
            } => {
                mark_avatar_uploaded(
                    context.conn(),
                    id,
                    *avatar_version,
                    *avatar_key,
                    *avatar_uploaded,
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
    // Keep transaction-scoped reservation on one repository error type.
    type Error = BaseError;

    // Reserve avatar key/version atomically inside the current transaction.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ReserveUserAvatar<'_>,
    ) -> BaseResult<UserAvatarReservation> {
        reserve_avatar(context.conn(), oper.id, oper.image_hash, oper.image_ext)
            .await
    }
}

impl Step<GetUserInfoExcluded<'_>, RdbContext> for RdbRepo {
    // Keep transaction-scoped exclusive reads on one repository error type.
    type Error = BaseError;

    // Read user row with lock for callers that mutate next.
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
    // Keep transaction-scoped deletion on one repository error type.
    type Error = BaseError;

    // Execute user deletion as part of ongoing transaction flow.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteUser<'_>,
    ) -> BaseResult<()> {
        delete(context.conn(), oper.id).await
    }
}
