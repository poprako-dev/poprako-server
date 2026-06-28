//! Data transfer objects for member use cases.

use serde::Deserialize;

use poprako_macro::Paginate;
use poprako_util::i18n::trl;

use crate::model::member::{MemberInfo, MemberListSpec};
use crate::model::role::{RoleField, RoleMask};
use crate::result::{ExpectedVariant, RootError, RootResult};
use crate::value::member::MemberInclOpt;

/// Presentation-ready membership information.
pub struct MemberInfoVal {
    pub id: String,

    pub user_id: String,
    pub user_nickname: String,

    pub team_id: String,

    pub roles: RoleMask,
}

impl From<MemberInfo> for MemberInfoVal {
    fn from(value: MemberInfo) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            user_nickname: value.user_nickname,
            team_id: value.team_id,
            roles: value.roles,
        }
    }
}

/// Input parameters for creating a member.
pub struct CreateMemberData {
    pub user_id: String,
    pub team_id: String,

    pub role_mask: RoleMask,
}

/// Return value from creating a member.
pub struct CreateMemberVal {
    pub id: String,
}

/// Input parameters for listing members by team.
#[Paginate]
#[derive(Deserialize)]
pub struct ListMemberInfosData {
    pub owner_id: Option<String>,

    pub team_id: Option<String>,

    pub user_nickname_keyword: Option<String>,
    pub role_bit: Option<RoleField>,

    #[serde(default)]
    pub incl_opt: Vec<MemberInclOpt>,
}

impl TryInto<MemberListSpec> for ListMemberInfosData {
    type Error = RootError;

    fn try_into(self) -> RootResult<MemberListSpec> {
        let invalid_args_err = || RootError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-team-or-user-required"),
        };

        if self.owner_id.is_some() == self.team_id.is_some() {
            return Err(invalid_args_err());
        }

        if self.user_nickname_keyword.is_some()
            || self.owner_id.is_some() && self.role_bit.is_some()
        {
            return Err(invalid_args_err());
        }

        if let Some(owner_id) = self.owner_id {
            return Ok(MemberListSpec::User {
                owner_id,
                incl_opt: self.incl_opt,
                offset: self.offset,
                limit: self.limit,
            });
        }

        Ok(MemberListSpec::Team {
            team_id: self.team_id.ok_or_else(invalid_args_err)?,
            role_bit: self.role_bit,
            incl_opt: self.incl_opt,
            offset: self.offset,
            limit: self.limit,
        })
    }
}

/// Input parameters for updating a member role mask.
pub struct UpdateMemberRoleData {
    pub id: String,
    pub roles: RoleMask,
}
