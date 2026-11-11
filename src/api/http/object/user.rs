//! HTTP wire objects for the user domain.
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

#[cfg(not(debug_assertions))]
use veil::Redact;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::api::http::object::upload::ImageUploadSlotObj;
use crate::api::http::result::Wire;
use crate::data::instr::user::{
    AllocUserAvatarInstr, MarkUserAvatarUploadedInstr, UpdateUserInfoInstr,
    UpdateUserPasswordInstr,
};
use crate::data::val::user::AllocUserAvatarVal;
use crate::data::view::obj_url::ObjUrlView;
use crate::data::view::user::UserInfoView;
use crate::value::image::{ImageExt, ImageHash};

/// Request body for allocating a user avatar upload.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocUserAvatarBody {
    /// SHA-256 identity of the exact avatar bytes.
    pub image_hash: ImageHash,
    /// Upload size used for validation and PUT signing.
    pub new_byte_len: u64,
    /// Avatar file format.
    pub ext: ImageExt,
}

/// Request body for confirming one allocated user avatar version.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkUserAvatarUploadedBody {
    /// Version returned in the avatar upload slot.
    #[serde(rename = "image_version")]
    pub image_ver: u32,
}

/// Request body for updating a user's profile.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateUserInfoBody {
    /// User identifier to update.
    pub id: String,

    /// Updated qualified identifier for login.
    pub qid: String,
    /// Updated display nickname.
    pub nickname: String,
}

/// Request body for replacing the authenticated user's password.
#[derive(Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(not(debug_assertions), derive(Redact))]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateUserPasswordBody {
    /// Current password for verification before change.
    #[cfg_attr(not(debug_assertions), redact(fixed = 10, with = "[REDACTED]"))]
    pub current_password: String,
    /// Desired new password.
    #[cfg_attr(not(debug_assertions), redact(fixed = 10, with = "[REDACTED]"))]
    pub new_password: String,
}

/// Presentation-ready user profile payload.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UserInfoObj {
    /// Unique user identifier.
    pub id: String,

    /// User display nickname.
    pub nickname: String,
    /// Unique qualified identifier used for login lookup.
    pub qid: String,

    /// Resolved signed download URL for the avatar image, or [`None`] if
    /// no avatar has been uploaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// Resolved signed download URL for the avatar thumbnail, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_thumbnail_url: Option<String>,

    /// Whether this user has super-admin privileges.
    pub is_sadmin: bool,
    /// Timestamp of the user's most recent activity, in milliseconds since Unix epoch.
    pub last_active_at: i64,

    /// Timestamp of user account creation, in milliseconds since Unix epoch.
    pub created_at: i64,
    /// Timestamp of the last profile update, in milliseconds since Unix epoch.
    pub updated_at: i64,
}

impl From<UserInfoView> for UserInfoObj {
    // Converts the presentation view into its wire representation.
    fn from(view: UserInfoView) -> Self {
        Self {
            id: view.id,
            nickname: view.nickname,
            qid: view.qid,
            avatar_url: view.avatar_url.map(ObjUrlView::into_string),
            avatar_thumbnail_url: view
                .avatar_thumbnail_url
                .map(ObjUrlView::into_string),
            is_sadmin: view.is_sadmin,
            last_active_at: view.last_active_at,
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}

impl Wire for UserInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = UserInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// User avatar upload allocation payload.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocUserAvatarPayload {
    /// Upload capability, absent when this avatar is already uploaded.
    pub slot: Option<ImageUploadSlotObj>,
}

impl From<AllocUserAvatarVal> for AllocUserAvatarPayload {
    // Converts the use-case result into its wire representation.
    fn from(val: AllocUserAvatarVal) -> Self {
        Self {
            slot: val.slot.map(Into::into),
        }
    }
}

impl Wire for AllocUserAvatarVal {
    // Serializable wire payload produced by this conversion.
    type Target = AllocUserAvatarPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

impl From<AllocUserAvatarBody> for AllocUserAvatarInstr {
    // Converts the wire body into the use-case input.
    fn from(body: AllocUserAvatarBody) -> Self {
        Self {
            image_hash: body.image_hash,
            new_byte_len: body.new_byte_len,
            ext: body.ext,
        }
    }
}

impl From<MarkUserAvatarUploadedBody> for MarkUserAvatarUploadedInstr {
    // Converts the wire body into the use-case input.
    fn from(body: MarkUserAvatarUploadedBody) -> Self {
        Self {
            image_ver: body.image_ver,
        }
    }
}

impl From<UpdateUserInfoBody> for UpdateUserInfoInstr {
    // Converts the wire body into the use-case input.
    fn from(body: UpdateUserInfoBody) -> Self {
        Self {
            id: body.id,
            qid: body.qid,
            nickname: body.nickname,
        }
    }
}

impl From<UpdateUserPasswordBody> for UpdateUserPasswordInstr {
    // Converts the wire body into the use-case input.
    fn from(body: UpdateUserPasswordBody) -> Self {
        Self {
            current_password: body.current_password,
            new_password: body.new_password,
        }
    }
}
