use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::user::UserComplex;
use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::write::user::{
    UserAvatarReservation, UserCredsRepl, UserEntry, UserInfoRepl,
};
use crate::part_impl::repo::rdb_impl::entity::user::{
    UserAspectRow, UserCredsRow, UserEntryRow, UserInfoRow,
};
use crate::part_impl::repo::rdb_impl::schema::t_user::dsl::*;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::RdbConn;
use crate::shared::result::{diesel, next_version};
use crate::value::image::{ImageExt, ImageHash};

#[instrument(level = "info", skip_all)]
/// Remove a user row from persistence.
pub async fn delete(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    // Execute hard delete and map DB errors to repository error type.
    diesel::delete(t_user.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Load credential material for authentication operations from the same backing row.
#[instrument(level = "info", skip_all)]
/// Load user credentials by QID.
pub async fn get_credential_by_qid(
    conn: &mut RdbConn,
    qid: &str,
) -> BaseRest<UserCredential> {
    //
    // Query only credential columns and convert them to user credential DTO.
    let row = t_user
        .filter(f_qid.eq(qid))
        .select(UserCredsRow::as_select())
        .get_result::<UserCredsRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let row = match row {
        //
        Some(row) => row,

        None => {
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
        }
    };

    accept(row.into())
}

// Return user info by QID, yielding `None` instead of an error when missing.
#[instrument(level = "info", skip_all)]
/// Find user info by QID, returning None when absent.
pub async fn find_info_by_qid(
    conn: &mut RdbConn,
    qid: &str,
) -> BaseRest<Option<UserInfo>> {
    //
    // Keep lookup soft-fail to allow callers to branch on existence.
    let row = t_user
        .filter(f_qid.eq(qid))
        .select(UserInfoRow::as_select())
        .get_result::<UserInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    row.map(TryInto::try_into).transpose()
}

// Insert a new user row and return the persisted info payload.
#[instrument(level = "info", skip_all)]
/// Insert a new user row from an entry.
pub async fn create(
    conn: &mut RdbConn,
    entry: &UserEntry,
) -> BaseRest<UserInfo> {
    //
    // Populate required identity and timestamp columns, then fetch created row.
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

// Update mutable identity fields (`qid`, `nickname`) for an existing user.
#[instrument(level = "info", skip_all)]
/// Apply a user info replacement.
pub async fn update_info(
    conn: &mut RdbConn,
    repl: &UserInfoRepl,
) -> BaseRest<()> {
    //
    // Apply one write that updates both fields and returns success when DB update succeeds.
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

// Replace a user's password hash and refresh the row-level update timestamp.
#[instrument(level = "info", skip_all)]
/// Update the user password hash.
pub async fn update_password_hash(
    conn: &mut RdbConn,
    repl: &UserCredsRepl,
) -> BaseRest<()> {
    //
    // Persist credential changes and bump `f_updated_at` in one SQL statement.
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

// Reserve an avatar object version, reusing existing key when hash unchanged.
#[instrument(level = "info", skip_all)]
/// Reserve an avatar key and version atomically.
pub async fn reserve_avatar(
    conn: &mut RdbConn,
    id: &str,
    image_hash: &ImageHash,
    image_ext: ImageExt,
) -> BaseRest<UserAvatarReservation> {
    //
    // Lock the target row, compare hash/ext, then either reuse or advance avatar version.
    let now = OffsetDateTime::now_utc();

    let (prev_key, uploaded, raw_version, stored_hash, stored_ext) = t_user
        .filter(f_id.eq(id))
        .select((
            f_avatar_key,
            f_avatar_uploaded,
            f_avatar_version,
            f_avatar_hash,
            f_avatar_extension,
        ))
        .for_update()
        .get_result::<(
            Option<String>,
            Option<bool>,
            Option<i64>,
            Option<Vec<u8>>,
            Option<String>,
        )>(conn)
        .await
        .map_err(diesel)?;

    let same_hash = prev_key.is_some()
        && stored_hash.as_deref() == Some(image_hash.as_bytes());

    if same_hash && stored_ext.as_deref() != Some(image_ext.suffix()) {
        //
        let message = trl("error-invalid-image-extension");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            user_id = %id,
            image_version = raw_version,
            stored_extension = ?stored_ext,
            requested_extension = %image_ext.suffix(),
            operation = "reserve user avatar",
            "expected user avatar error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    }

    if same_hash {
        //
        let object_key = prev_key.ok_or_else(|| BaseError::Unrecoverable {
            message: "[reserve_avatar] pending avatar key is missing".into(),
        })?;

        return accept(UserAvatarReservation {
            object_key,
            prev_object_key: None,
            avatar_version: u32::try_from(raw_version.ok_or_else(|| {
                //
                BaseError::Unrecoverable {
                    message: "[reserve_avatar] avatar version is missing"
                        .into(),
                }
            })?)
            .map_err(|_| BaseError::Unrecoverable {
                message: "[reserve_avatar] avatar version is invalid".into(),
            })?,
            is_upload_required: !uploaded.unwrap_or(false),
        });
    }

    let version = next_version(raw_version.unwrap_or(0))?;

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
        is_upload_required: true,
    })
}

// Mark upload result for an avatar with optimistic-version guarding.
#[instrument(level = "info", skip_all)]
/// Mark the user avatar as uploaded.
pub async fn mark_avatar_uploaded(
    conn: &mut RdbConn,
    id: &str,
    version: u32,
    avatar_key: Option<&str>,
    avatar_uploaded: bool,
) -> BaseRest<()> {
    //
    // Only write when version (and optional key) match; return mismatch error if stale.
    let now = OffsetDateTime::now_utc();

    let affected = match avatar_key {
        //
        Some(avatar_key) => {
            //
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
            //
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
        //
        let message = trl("error-avatar-version-mismatch");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            user_id = %id,
            image_version = version,
            avatar_key = ?avatar_key,
            operation = "mark user avatar uploaded",
            "expected user avatar version error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    }

    accept(())
}

// Touch `last_active_at` for heartbeat and usage tracking.
#[instrument(level = "info", skip_all)]
/// Update the user last-active timestamp.
pub async fn touch_last_active(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    // Keep access timestamp current for activity-driven features.
    let now = OffsetDateTime::now_utc();

    let aspect = UserAspectRow::new(now).last_active_at(now);

    diesel::update(t_user.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Load one user info row with `FOR UPDATE` lock for follow-up writes.
#[instrument(level = "info", skip_all)]
/// Load user info by ID, locking the row for update.
pub async fn get_info_by_id_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<UserInfo> {
    //
    // Use a row lock so later mutation in the same transaction is serialized.
    let row = t_user
        .filter(f_id.eq(id))
        .select(UserInfoRow::as_select())
        .for_update()
        .get_result::<UserInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let row = match row {
        //
        Some(row) => row,

        None => {
            //
            let message = trl("error-user-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %message,
                user_id = %id,
                operation = "lock user info",
                "expected user error",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message,
            });
        }
    };

    row.try_into()
}

// Load one user info row by primary key and map DB row into response model.
#[instrument(level = "info", skip_all)]
/// Load a single user info by ID.
pub async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<UserInfo> {
    //
    // Query `t_user` by `f_id`, fail with `error-user-not-found` when absent.
    let row = t_user
        .filter(f_id.eq(id))
        .select(UserInfoRow::as_select())
        .get_result::<UserInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let row = match row {
        //
        Some(row) => row,

        None => {
            //
            let message = trl("error-user-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %message,
                user_id = %id,
                operation = "get user info",
                "expected user error",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message,
            });
        }
    };

    row.try_into()
}
