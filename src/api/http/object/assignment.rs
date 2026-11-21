//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::api::http::object::chapter::ChapterInfoObj;
use crate::api::http::object::user::UserInfoObj;
use crate::api::http::result::Wire;
use crate::data::instr::assignment::{
    JoinChapterAssignmentInstr, ListAssignmentInfosInstr,
    UpdateAssignmentRolesInstr,
};
use crate::data::view::assignment::AssignmentInfoView;
use crate::value::assignment::AssignmentInclOpt;
use crate::value::pagination::PubListLimit;
use crate::value::role::{RoleField, RoleMask};

/// Input parameters for listing assignments by chapter or owner user.
///
/// Exactly one of `chapter_id` or `owner_id` is required:
/// - `chapter_id`: list assignments on that chapter;
/// - `owner_id`: list assignments owned by that user.
///
/// `role` optionally narrows by a single role bit in either mode. `incl`
/// embeds related rows; dotted values imply their parent segments.
///
/// Example: `/api/v1/assignments?chapter_id=c_1&role=1&incl=chapter.comic.workset.team&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListAssignmentInfosQuery {
    /// Chapter mode: list assignments on this chapter. Mutually exclusive with
    /// `owner_id`.
    pub chapter_id: Option<String>,

    /// Owner-user mode: list assignments owned by this user. Mutually exclusive
    /// with `chapter_id`.
    pub owner_id: Option<String>,

    /// Single worker role-bit filter. ADMIN and BOT are rejected; composite
    /// values are rejected.
    pub role: Option<RoleField>,

    /// Related rows to embed. Repeatable. Values: `user`, `chapter`,
    /// `chapter.comic`, `chapter.comic.workset`, `chapter.comic.workset.team`,
    /// `chapter.creator`, `chapter.comic.creator`. Dotted values imply their
    /// parent segments.
    #[serde(default)]
    pub incl: Vec<AssignmentInclOpt>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

impl From<ListAssignmentInfosQuery> for ListAssignmentInfosInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListAssignmentInfosQuery) -> Self {
        Self {
            chapter_id: value.chapter_id,
            owner_id: value.owner_id,
            role: value.role,
            incl: value.incl,
            offset: value.offset,
            limit: value.limit,
        }
    }
}

/// Input parameters for updating assignment roles.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateAssignmentRolesBody {
    /// Target chapter identifier.
    pub chapter_id: String,
    /// Target user identifier.
    pub user_id: String,

    /// Non-empty worker role mask to apply; ADMIN is not assignable.
    pub roles: RoleMask,
}

impl From<UpdateAssignmentRolesBody> for UpdateAssignmentRolesInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UpdateAssignmentRolesBody) -> Self {
        Self {
            chapter_id: value.chapter_id,
            user_id: value.user_id,
            roles: value.roles,
        }
    }
}

/// Input parameters for a user joining a chapter as a worker via role
/// selection.
///
/// The role mask must contain role bits that are valid for volunteer
/// assignment; the use case layer validates this before applying.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct JoinChapterAssignmentBody {
    /// Chapter to join as an assignee.
    pub chapter_id: String,

    /// Volunteer role mask.
    pub roles: RoleMask,
}

impl From<JoinChapterAssignmentBody> for JoinChapterAssignmentInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: JoinChapterAssignmentBody) -> Self {
        Self {
            chapter_id: value.chapter_id,
            roles: value.roles,
        }
    }
}

/// Presentation-ready chapter assignment information.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AssignmentInfoObj {
    /// Unique identifier of the assignment.
    pub id: String,

    /// Owning chapter identifier.
    pub chapter_id: String,
    /// Assigned user identifier.
    pub user_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Resolved user information, when included.
    pub user: Option<UserInfoObj>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Resolved chapter information, when included.
    pub chapter: Option<ChapterInfoObj>,

    /// Worker role mask assigned to this user for the chapter; excludes ADMIN.
    pub roles: RoleMask,

    /// Timestamp of creation in milliseconds.
    pub created_at: i64,
    /// Timestamp of last update in milliseconds.
    pub updated_at: i64,
}

impl From<AssignmentInfoView> for AssignmentInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AssignmentInfoView) -> Self {
        Self {
            id: value.id,
            chapter_id: value.chapter_id,
            user_id: value.user_id,
            user: value.user.map(std::convert::Into::into),
            chapter: value.chapter.map(std::convert::Into::into),
            roles: value.roles,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl Wire for AssignmentInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = AssignmentInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}
