//! Data transfer objects for user profile use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli;

use crate::model::user::UserInfo;
use crate::part::image::ImagePool;
use crate::result::{BaseResult, accept};

/// Presentation-ready user profile information.
///
/// Converts the raw [`UserInfo`] timestamps to Unix milliseconds and
/// resolves the avatar key to a signed download URL via [`ImagePool`] when
/// the avatar has been uploaded.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UserInfoVal {
    pub id: String,

    pub nickname: String,
    pub qid: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_thumbnail_url: Option<String>,

    pub is_sadmin: bool,
    pub last_active_at: i64,

    pub created_at: i64,
    pub updated_at: i64,
}

impl UserInfoVal {
    /// Converts a [`UserInfo`] into a presentation-ready value.
    ///
    /// Resolves a signed avatar download URL when the avatar has
    /// been uploaded and a key is present. Timestamps are converted
    /// from [`OffsetDateTime`] to Unix milliseconds.
    ///
    /// [`OffsetDateTime`]: time::OffsetDateTime
    /// [`UserInfo`]: crate::model::user::UserInfo
    pub async fn from_model<P>(
        image_pool: &P,
        model: UserInfo,
    ) -> BaseResult<Self>
    where
        P: ImagePool,
    {
        let (avatar_url, avatar_thumbnail_url) =
            match (model.avatar_uploaded, &model.avatar_key) {
                //
                (true, Some(key)) => (
                    image_pool.gen_download_url(key).await.ok(),
                    image_pool.gen_thumbnail_download_url(key).await.ok(),
                ),

                _ => (None, None),
            };

        accept(Self {
            id: model.id,
            nickname: model.nickname,
            qid: model.qid,
            avatar_url: avatar_url.map(Into::into),
            avatar_thumbnail_url: avatar_thumbnail_url.map(Into::into),
            is_sadmin: model.is_sadmin,
            last_active_at: model.last_active_at.to_unix_milli(),
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        })
    }
}

/// Input parameters for updating a user's profile.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UpdateUserInfoParams {
    pub id: String,

    pub qid: String,
    pub nickname: String,
}

/// Input parameters for replacing the authenticated user's password.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UpdateUserPasswordParams {
    pub current_password: String,
    pub new_password: String,
}

/// Input parameters for reserving a new avatar upload slot.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ReserveUserAvatarParams {
    pub file_ext: String,
}

/// Return value from a successful avatar reservation.
///
/// The client uses `put_url` to upload the avatar image directly to object
/// storage. `avatar_version` must be echoed back when confirming the upload.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ReserveUserAvatarPayload {
    pub put_url: String,
    pub avatar_version: u32,
}

/// Input parameters for confirming an avatar upload completed.
///
/// `avatar_version` must match the version returned by the reservation step.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct MarkUserAvatarUploadedParams {
    pub avatar_version: u32,
}
