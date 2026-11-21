//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::api::http::object::upload::ImageUploadSlotObj;
use crate::api::http::result::Wire;
use crate::data::instr::team::{
    AllocTeamAvatarInstr, CreateTeamInstr, ListTeamInfosInstr,
    MarkTeamAvatarUploadedInstr, UpdateTeamInfoInstr,
};
use crate::data::val::team::AllocTeamAvatarVal;
use crate::data::view::obj_url::ObjUrlView;
use crate::data::view::team::TeamInfoView;
use crate::value::image::{ImageExt, ImageHash};
use crate::value::pagination::PubListLimit;

/// Request to allocate a team avatar upload.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocTeamAvatarBody {
    /// SHA-256 identity of the exact avatar bytes.
    pub image_hash: ImageHash,
    /// Upload size used for validation and PUT signing.
    pub new_byte_len: u64,
    /// Avatar file format.
    pub ext: ImageExt,
}

impl From<AllocTeamAvatarBody> for AllocTeamAvatarInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AllocTeamAvatarBody) -> Self {
        Self {
            image_hash: value.image_hash,
            new_byte_len: value.new_byte_len,
            ext: value.ext,
        }
    }
}

/// Request to confirm one allocated team avatar version.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkTeamAvatarUploadedBody {
    /// Version returned in the avatar upload slot.
    #[serde(rename = "image_version")]
    pub image_ver: u32,
}

impl From<MarkTeamAvatarUploadedBody> for MarkTeamAvatarUploadedInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: MarkTeamAvatarUploadedBody) -> Self {
        Self {
            image_ver: value.image_ver,
        }
    }
}

/// Input parameters for creating a new team.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateTeamBody {
    /// Team display name.
    pub name: String,
    /// Team description text.
    pub description: String,
}

impl From<CreateTeamBody> for CreateTeamInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateTeamBody) -> Self {
        Self {
            name: value.name,
            description: value.description,
        }
    }
}

/// Input parameters for listing teams.
///
/// Exactly one listing mode applies based on `user_id`:
/// - `user_id` omitted: list all teams (requires super-admin, otherwise
///   `403`);
/// - `user_id` present: list teams the authenticated user has joined. A
///   different user's ID is rejected with `403`.
///
/// Example: `/api/v1/teams?user_id=u_123&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListTeamInfosQuery {
    /// Filter to teams joined by the authenticated user. A different user's
    /// ID is rejected. Omit to list all teams (super-admin only).
    pub user_id: Option<String>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

impl From<ListTeamInfosQuery> for ListTeamInfosInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListTeamInfosQuery) -> Self {
        Self {
            user_id: value.user_id,
            offset: value.offset,
            limit: value.limit,
        }
    }
}

/// Input parameters for updating a team's profile.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateTeamInfoBody {
    /// Team identifier.
    pub id: String,

    /// Updated team display name.
    pub name: String,
    /// Updated team description text.
    pub description: String,
}

impl From<UpdateTeamInfoBody> for UpdateTeamInfoInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UpdateTeamInfoBody) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
        }
    }
}

/// Team avatar upload allocation response.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocTeamAvatarPayload {
    /// Upload capability, absent when this avatar is already uploaded.
    pub slot: Option<ImageUploadSlotObj>,
}

impl From<AllocTeamAvatarVal> for AllocTeamAvatarPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AllocTeamAvatarVal) -> Self {
        Self {
            slot: value.slot.map(std::convert::Into::into),
        }
    }
}

impl Wire for AllocTeamAvatarVal {
    // Serializable wire payload produced by this conversion.
    type Target = AllocTeamAvatarPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Presentation-ready team profile information.
///
/// Converts raw [`TeamInfo`] timestamps to Unix milliseconds and accepts an
/// avatar origin and thumbnail URLs already resolved by the use-case layer.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct TeamInfoObj {
    /// Unique team identifier.
    pub id: String,

    /// Team display name.
    pub name: String,
    /// Team description text.
    pub description: String,

    /// Signed avatar download URL, when one has been uploaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "swagger", schema(value_type = Option<String>))]
    pub avatar_url: Option<String>,
    /// Signed avatar-thumbnail download URL, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "swagger", schema(value_type = Option<String>))]
    pub avatar_thumbnail_url: Option<String>,

    /// Timestamp of creation, in Unix milliseconds.
    pub created_at: i64,
    /// Timestamp of last update, in Unix milliseconds.
    pub updated_at: i64,
}

impl From<TeamInfoView> for TeamInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: TeamInfoView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            avatar_url: value.avatar_url.map(ObjUrlView::into_string),
            avatar_thumbnail_url: value
                .avatar_thumbnail_url
                .map(ObjUrlView::into_string),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl Wire for TeamInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = TeamInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}
