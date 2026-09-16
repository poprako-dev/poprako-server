//! Pure rules for workset entities.

/// Pure permission rules.
pub mod perm {
    use crate::complex::util::{
        check_user_is_team_admin, check_user_is_team_member,
    };
    use crate::model::read::proj::member::MemberInfo;
    use crate::result::BaseRest;

    /// Verify the caller is a team admin.
    pub fn ensure_user_can_create(member_info: &MemberInfo) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }

    /// Verify the caller is a team member.
    pub const fn ensure_user_can_list_infos(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_member(member_info)
    }

    /// Verify the caller may read the workset.
    pub const fn ensure_user_can_get_info(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
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

use crate::util::next_snowflake_id;

/// Generate a unique, time-ordered workset identifier backed by a snowflake value.
pub fn gen_id() -> String {
    next_snowflake_id()
}
