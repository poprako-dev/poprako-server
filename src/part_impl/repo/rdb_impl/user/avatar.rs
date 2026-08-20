//! User avatar reservation and upload-state persistence.

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::user::UserComplex;
use crate::model::write::user::UserAvatarReservation;
use crate::part_impl::repo::rdb_impl::schema::t_user::dsl::{
    f_avatar_extension, f_avatar_hash, f_avatar_key, f_avatar_uploaded,
    f_avatar_version, f_id, f_updated_at, t_user,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::RdbConn;
use crate::shared::result::{diesel, next_version};
use crate::value::image::{ImageExt, ImageHash};

#[instrument(level = "info", skip_all)]
/// Reserve an avatar key and version atomically.
pub async fn reserve_avatar(
    conn: &mut RdbConn,
    id: &str,
    image_hash: &ImageHash,
    image_ext: ImageExt,
) -> BaseRest<UserAvatarReservation> {
    //
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
