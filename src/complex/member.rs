//! Complex domain logic for [Member] aggregates — ID generation and perm gates.

#[cfg(test)]
mod tests;

use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_member,
};
use crate::model::read::proj::member::MemberInfo;
use crate::result::BaseRest;
use crate::util::next_snowflake_id;
use crate::value::role::{RoleField, RoleMask};

/// Domain opers for [Member] aggregates: unique identifier generation.
pub struct MemberComplex;

impl MemberComplex {
    /// Generates a unique member identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Checks whether a team retains an admin after one member role update.
    pub fn team_has_admin_after_role_update(
        member_infos: &[MemberInfo],
        subject_member_info: &MemberInfo,
        roles: RoleMask,
    ) -> bool {
        //
        member_infos.iter().any(|member_info| {
            //
            if member_info.id == subject_member_info.id {
                roles.has_any_role(&[RoleField::ADMIN])
            } else {
                member_info.roles.has_any_role(&[RoleField::ADMIN])
            }
        })
    }

    /// Checks whether a team retains an admin after one member deletion.
    pub fn team_has_admin_after_delete(
        member_infos: &[MemberInfo],
        subject_member_info: &MemberInfo,
    ) -> bool {
        //
        member_infos.iter().any(|member_info| {
            //
            member_info.id != subject_member_info.id
                && member_info.roles.has_any_role(&[RoleField::ADMIN])
        })
    }
}

/// perm-gate opers for team membership — team-scoped.
pub struct MemberPermComplex;

impl MemberPermComplex {
    /// Verify the caller is a team admin of the given team.
    pub fn ensure_user_can_update_info(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }

    /// Verify the caller is a team admin of the given team.
    pub fn ensure_user_can_delete(member_info: &MemberInfo) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }

    /// Verify the caller is a team admin of the given team.
    pub fn ensure_user_can_create(member_info: &MemberInfo) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }

    /// Verify the caller is a teammember.
    pub const fn ensure_user_can_list_infos(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_member(member_info)
    }
}
