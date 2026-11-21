//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::api::http::object::user::UserInfoObj;
use crate::api::http::result::Wire;
use crate::data::instr::comment::{CreateCommentInstr, ListCommentInfosInstr};
use crate::data::val::comment::CreateCommentVal;
use crate::data::view::comment::CommentInfoView;
use crate::value::comment::CommentInclOpt;
use crate::value::pagination::PubListLimit;

/// Input parameters for listing comments.
///
/// `incl` embeds related rows into each item.
///
/// Example: `/api/v1/teams/{team_id}/comments?incl=user&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListCommentInfosQuery {
    /// Parent team whose comments to list.
    pub team_id: String,

    /// Related rows to embed. Repeatable. Values: `user`.
    #[serde(default)]
    pub incl: Vec<CommentInclOpt>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

impl From<ListCommentInfosQuery> for ListCommentInfosInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListCommentInfosQuery) -> Self {
        Self {
            team_id: value.team_id,
            incl: value.incl,
            offset: value.offset,
            limit: value.limit,
        }
    }
}

/// Input parameters for creating a comment.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateCommentBody {
    /// Target team identifier.
    pub team_id: String,
    /// Comment body text.
    pub content: String,
}

impl From<CreateCommentBody> for CreateCommentInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateCommentBody) -> Self {
        Self {
            team_id: value.team_id,
            content: value.content,
        }
    }
}

/// Return value from creating a comment.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateCommentPayload {
    /// Identifier of the newly created comment.
    pub id: String,
}

impl From<CreateCommentVal> for CreateCommentPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateCommentVal) -> Self {
        Self { id: value.id }
    }
}

impl Wire for CreateCommentVal {
    // Serializable wire payload produced by this conversion.
    type Target = CreateCommentPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Presentation-ready team board comment information.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CommentInfoObj {
    /// Unique comment identifier.
    pub id: String,

    /// Owning team identifier.
    pub team_id: String,
    /// Authoring user identifier.
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Author user information, present when requested via inclusion options.
    pub user: Option<UserInfoObj>,

    /// Comment body text.
    pub content: String,

    /// Timestamp of creation in milliseconds since Unix epoch.
    pub created_at: i64,
}

impl From<CommentInfoView> for CommentInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CommentInfoView) -> Self {
        Self {
            id: value.id,
            team_id: value.team_id,
            user_id: value.user_id,
            user: value.user.map(std::convert::Into::into),
            content: value.content,
            created_at: value.created_at,
        }
    }
}

impl Wire for CommentInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = CommentInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Query for listing comments within a team.
///
/// `incl` embeds related rows into each item.
///
/// Example: `?incl=user&offset=0&limit=20`.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct CommentListQuery {
    /// Related rows to embed. Repeatable. Values: `user`.
    #[serde(default)]
    pub incl: Vec<CommentInclOpt>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: PubListLimit,
}
