//! Complex domain logic for [Member] aggregates — ID generation and perm gates.
use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_member,
};
use crate::model::read::proj::member::MemberInfo;
use crate::result::BaseRest;
use crate::util::next_snowflake_id;

/// Domain opers for [Member] aggregates: unique identifier generation.
pub struct MemberComplex;

impl MemberComplex {
    /// Generates a unique member identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
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
    pub fn ensure_user_can_list_infos(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_member(member_info)
    }
}
