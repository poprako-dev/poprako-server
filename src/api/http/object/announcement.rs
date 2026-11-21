//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::api::http::object::user::UserInfoObj;
use crate::api::http::result::Wire;
use crate::data::instr::announcement::{
    CreateAnnouncementInstr, ListAnnouncementInfosInstr,
    UpdateAnnouncementInfoInstr,
};
use crate::data::val::announcement::CreateAnnouncementVal;
use crate::data::view::announcement::AnnouncementInfoView;
use crate::value::announcement::AnnouncementInclOpt;
use crate::value::pagination::PubListLimit;

/// Input parameters for listing announcements.
///
/// `incl` embeds related rows into each item.
///
/// Example: `/api/v1/announcements?team_id=t_1&incl=user&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListAnnouncementInfosQuery {
    /// Parent team whose announcements to list.
    pub team_id: String,

    /// Related rows to embed. Repeatable. Values: `user`.
    #[serde(default)]
    pub incl: Vec<AnnouncementInclOpt>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

impl From<ListAnnouncementInfosQuery> for ListAnnouncementInfosInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListAnnouncementInfosQuery) -> Self {
        Self {
            team_id: value.team_id,
            incl: value.incl,
            offset: value.offset,
            limit: value.limit,
        }
    }
}

/// Input parameters for creating an announcement.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateAnnouncementBody {
    /// Target team identifier.
    pub team_id: String,

    /// Announcement title.
    pub title: String,
    /// Announcement body content.
    pub content: String,
}

impl From<CreateAnnouncementBody> for CreateAnnouncementInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateAnnouncementBody) -> Self {
        Self {
            team_id: value.team_id,
            title: value.title,
            content: value.content,
        }
    }
}

/// Input parameters for replacing an announcement's editable fields.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateAnnouncementInfoBody {
    /// Announcement identifier to update.
    pub id: String,

    /// Updated announcement title.
    pub title: String,
    /// Updated announcement body content.
    pub content: String,
}

impl From<UpdateAnnouncementInfoBody> for UpdateAnnouncementInfoInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UpdateAnnouncementInfoBody) -> Self {
        Self {
            id: value.id,
            title: value.title,
            content: value.content,
        }
    }
}

/// Return value from creating an announcement.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateAnnouncementPayload {
    /// Identifier of the created announcement.
    pub id: String,
}

impl From<CreateAnnouncementVal> for CreateAnnouncementPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateAnnouncementVal) -> Self {
        Self { id: value.id }
    }
}

impl Wire for CreateAnnouncementVal {
    // Serializable wire payload produced by this conversion.
    type Target = CreateAnnouncementPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Presentation-ready team announcement information.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AnnouncementInfoObj {
    /// Unique identifier.
    pub id: String,

    /// Owning team identifier.
    pub team_id: String,
    /// Authoring user identifier.
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Authoring user information, omitted when not requested.
    pub user: Option<UserInfoObj>,

    /// Announcement title.
    pub title: String,
    /// Announcement body content.
    pub content: String,

    /// Timestamp of creation in Unix milliseconds.
    pub created_at: i64,
}

impl From<AnnouncementInfoView> for AnnouncementInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AnnouncementInfoView) -> Self {
        Self {
            id: value.id,
            team_id: value.team_id,
            user_id: value.user_id,
            user: value.user.map(std::convert::Into::into),
            title: value.title,
            content: value.content,
            created_at: value.created_at,
        }
    }
}

impl Wire for AnnouncementInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = AnnouncementInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Query for listing announcements within a team.
///
/// `incl` embeds related rows into each item.
///
/// Example: `?incl=user&offset=0&limit=20`.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct AnnouncementListQuery {
    /// Related rows to embed. Repeatable. Values: `user`.
    #[serde(default)]
    pub incl: Vec<AnnouncementInclOpt>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: PubListLimit,
}
