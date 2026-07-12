//! Data transfer objects for user profile use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli;

use crate::model::user_model;
use crate::part::image::ImagePool;
use crate::result::RegularResult;

/// Presentation-ready user profile information.
///
/// Converts the raw [`UserInfoModel`] timestamps to Unix milliseconds and
/// resolves the avatar key to a signed download URL via [`ImagePool`] when
/// the avatar has been uploaded.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct InfoVal {
    pub id: String,

    pub nickname: String,
    pub qid: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub is_sadmin: bool,
    pub last_active_at: i64,

    pub created_at: i64,
    pub updated_at: i64,
}

impl InfoVal {
    /// Converts a [`UserInfoModel`] into a presentation-ready value.
    ///
    /// Resolves a signed avatar download URL when the avatar has
    /// been uploaded and a key is present. Timestamps are converted
    /// from [`OffsetDateTime`] to Unix milliseconds.
    ///
    /// [`OffsetDateTime`]: time::OffsetDateTime
    pub async fn from_model<P>(
        image_pool: &P,
        model: user_model::Info,
    ) -> RegularResult<Self>
    where
        P: ImagePool,
    {
        let avatar_url = match (model.avatar_uploaded, &model.avatar_key) {
            //
            (true, Some(key)) => image_pool.get_signed(key).await.ok(),

            _ => None,
        };

        Ok(Self {
            id: model.id,
            nickname: model.nickname,
            qid: model.qid,
            avatar_url: avatar_url.map(Into::into),
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
pub struct UpdateInfoData {
    pub id: String,

    pub qid: String,
    pub nickname: String,
}

/// Input parameters for reserving a new avatar upload slot.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ReserveAvatarData {
    pub file_ext: String,
}

/// Return value from a successful avatar reservation.
///
/// The client uses `put_url` to upload the avatar image directly to object
/// storage. `avatar_version` must be echoed back when confirming the upload.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ReserveAvatarVal {
    pub put_url: String,
    pub avatar_version: i64,
}

/// Input parameters for confirming an avatar upload completed.
///
/// `avatar_version` must match the version returned by the reservation step.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct MarkAvatarUploadedData {
    pub avatar_version: i64,
}
