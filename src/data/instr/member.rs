//! Instr DTOs for the member domain.

//! Data transfer objects for member use cases.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use poprako_util::i18n::trl;

use crate::model::read::spec::member::MemberListSpec;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::member::MemberInclOpt;
use crate::value::pagination::PubListLimit;
use crate::value::role::{RoleField, RoleMask};

/// Input parameters for creating a member.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateMemberInstr {
    //
    /// User identifier for the new membership.
    pub user_id: String,
    /// Team identifier for the new membership.
    pub team_id: String,

    /// Initial role bitmask for the member.
    pub roles: RoleMask,
}

/// Input parameters for joining a team through a member invitation.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct JoinTeamInstr {
    /// Invitation code to join the team.
    pub code: String,
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
pub struct ListMemberInfosInstr {
    //
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
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<MemberInclOpt>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

impl TryInto<MemberListSpec> for ListMemberInfosInstr {
    // The error type for invalid member listing parameters.
    type Error = BaseError;

    // Convert validated member query parameters into the domain list spec.
    fn try_into(self) -> BaseRest<MemberListSpec> {
        //
        let Self {
            owner_id,
            team_id,
            fuzzy_nickname,
            role,
            incl_opt,
            offset,
            limit,
        } = self;

        if owner_id.is_some() == team_id.is_some() {
            //
            let err_message = trl("error-team-or-user-required");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                owner_id = ?owner_id,
                team_id = ?team_id,
                role = ?role,
                fuzzy_nickname = ?fuzzy_nickname,
                "expected error: member list requires one scope identifier",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        if owner_id.is_some() && role.is_some() {
            //
            let err_message = trl("error-team-or-user-required");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                owner_id = ?owner_id,
                team_id = ?team_id,
                role = ?role,
                fuzzy_nickname = ?fuzzy_nickname,
                "expected error: member list owner scope cannot filter by role",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        if let Some(owner_id) = owner_id {
            //
            return accept(MemberListSpec::User {
                owner_id,
                incl_opt,
                offset,
                limit,
            });
        }

        let Some(team_id) = team_id else {
            //
            let err_message = trl("error-team-or-user-required");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                owner_id = ?owner_id,
                team_id = ?team_id,
                role = ?role,
                fuzzy_nickname = ?fuzzy_nickname,
                "expected error: member list team scope is missing",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        };

        accept(MemberListSpec::Team {
            team_id,
            fuzzy_nickname,
            role,
            incl_opt,
            offset,
            limit,
        })
    }
}

/// Input parameters for updating a member's roles.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateMemberRolesInstr {
    //
    /// Member identifier to update.
    pub id: String,
    /// New role bitmask to assign.
    pub roles: RoleMask,
}
