//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

#[cfg(not(debug_assertions))]
use veil::Redact;

use crate::api::http::object::team::TeamInfoObj;
use crate::api::http::object::user::UserInfoObj;
use crate::api::http::result::Wire;
use crate::data::instr::member::{
    CreateMemberInstr, JoinTeamInstr, ListMemberInfosInstr,
    UpdateMemberRolesInstr,
};
use crate::data::val::member::CreateMemberVal;
use crate::data::view::member::MemberInfoView;
use crate::value::member::MemberInclOpt;
use crate::value::pagination::PubListLimit;
use crate::value::role::{RoleField, RoleMask};

/// Input parameters for creating a member.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateMemberBody {
    /// User identifier for the new membership.
    pub user_id: String,
    /// Team identifier for the new membership.
    pub team_id: String,

    /// Initial role bitmask for the member.
    pub roles: RoleMask,
}

impl From<CreateMemberBody> for CreateMemberInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateMemberBody) -> Self {
        Self {
            user_id: value.user_id,
            team_id: value.team_id,
            roles: value.roles,
        }
    }
}

/// Input parameters for joining a team through a member invitation.
#[derive(Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(not(debug_assertions), derive(Redact))]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct JoinTeamBody {
    /// Invitation code to join the team.
    #[cfg_attr(not(debug_assertions), redact(fixed = 10, with = "[REDACTED]"))]
    pub code: String,
}

impl From<JoinTeamBody> for JoinTeamInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: JoinTeamBody) -> Self {
        Self { code: value.code }
    }
}

/// Input parameters for listing members by team.
///
/// Exactly one of `owner_id` or `team_id` is required:
/// - `owner_id`: list teams/memberships owned by that user; `role` and
///   `fuzzy_nickname` must be omitted in this mode;
/// - `team_id`: list members of that team, optionally narrowed by
///   `fuzzy_nickname` and/or `role`.
///
/// `incl` embeds related rows (`user`, `team`).
///
/// Example: `/api/v1/members?team_id=t_1&fuzzy_nickname=al&role=1&incl=user&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListMemberInfosQuery {
    /// Owner-user mode: list teams/memberships owned by this user. Mutually
    /// exclusive with `team_id`; when set, `role` and `fuzzy_nickname` must be
    /// omitted.
    pub owner_id: Option<String>,

    /// Team mode: list members of this team. Mutually exclusive with
    /// `owner_id`.
    pub team_id: Option<String>,

    /// Substring filter on member nickname (team mode only).
    pub fuzzy_nickname: Option<String>,

    /// Single role-bit filter (team mode only). Must be a singular valid role
    /// bit; composite values are rejected.
    pub role: Option<RoleField>,

    /// Related rows to embed. Repeatable. Values: `user`, `team`.
    #[serde(default)]
    pub incl: Vec<MemberInclOpt>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

impl From<ListMemberInfosQuery> for ListMemberInfosInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListMemberInfosQuery) -> Self {
        Self {
            owner_id: value.owner_id,
            team_id: value.team_id,
            fuzzy_nickname: value.fuzzy_nickname,
            role: value.role,
            incl: value.incl,
            offset: value.offset,
            limit: value.limit,
        }
    }
}

/// Input parameters for updating a member's roles.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateMemberRolesBody {
    /// Member identifier to update.
    pub id: String,
    /// New role bitmask to assign.
    pub roles: RoleMask,
}

impl From<UpdateMemberRolesBody> for UpdateMemberRolesInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UpdateMemberRolesBody) -> Self {
        Self {
            id: value.id,
            roles: value.roles,
        }
    }
}

/// Return value from creating a member.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateMemberPayload {
    /// Identifier of the created member.
    pub id: String,
}

impl From<CreateMemberVal> for CreateMemberPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateMemberVal) -> Self {
        Self { id: value.id }
    }
}

impl Wire for CreateMemberVal {
    // Serializable wire payload produced by this conversion.
    type Target = CreateMemberPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Presentation-ready membership information.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MemberInfoObj {
    /// Unique member identifier.
    pub id: String,

    /// Owning user identifier.
    pub user_id: String,
    /// Display nickname of the member.
    pub nickname: String,
    /// Unix timestamp of the last activity, in milliseconds.
    pub last_active_at: i64,

    /// Team identifier this membership belongs to.
    pub team_id: String,

    /// Resolved user detail, present when the `user` include option is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserInfoObj>,
    /// Resolved team detail, present when the `team` include option is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<TeamInfoObj>,

    /// Bitmask of roles assigned to this member.
    pub roles: RoleMask,
}

impl From<MemberInfoView> for MemberInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: MemberInfoView) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            nickname: value.nickname,
            last_active_at: value.last_active_at,
            team_id: value.team_id,
            user: value.user.map(std::convert::Into::into),
            team: value.team.map(std::convert::Into::into),
            roles: value.roles,
        }
    }
}

impl Wire for MemberInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = MemberInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Query for the current-user memberships list endpoint (`/members/me`).
///
/// `incl` embeds related rows into each item.
///
/// Example: `?incl=user&incl=team&offset=0&limit=20`.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
#[cfg_attr(test, derive(Debug))]
pub struct MemberMeListQuery {
    /// Related rows to embed. Repeatable. Values: `user`, `team`.
    #[serde(default)]
    pub incl: Vec<MemberInclOpt>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: PubListLimit,
}
