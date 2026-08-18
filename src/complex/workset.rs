//! Pure rules for workset entities.

use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_member,
};
use crate::model::read::proj::member::MemberInfo;
use crate::result::BaseRest;
use crate::util::next_snowflake_id;

/// Pure domain operations for workset entities.
pub struct WorksetComplex;

impl WorksetComplex {
    /// Generate a unique, time-ordered workset identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }
}

/// Pure permission rules for workset entities.
pub struct WorksetPermComplex;

impl WorksetPermComplex {
    /// Verify the caller is a team admin.
    pub fn ensure_user_can_create(member_info: &MemberInfo) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }

    /// Verify the caller is a team member.
    pub fn ensure_user_can_list_infos(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_member(member_info)
    }

    /// Verify the caller may read the workset.
    pub fn ensure_user_can_get_info(member_info: &MemberInfo) -> BaseRest<()> {
        check_user_is_team_member(member_info)
    }

    /// Verify the caller may update the workset.
    pub fn ensure_user_can_update_info(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }

    /// Verify the caller may delete the workset.
    pub fn ensure_user_can_delete(member_info: &MemberInfo) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }
}
