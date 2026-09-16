//! Complex-domain opers for team comments.

/// Pure permission rules.
pub mod perm {
    use crate::complex::util::check_user_is_team_member;
    use crate::model::read::proj::member::MemberInfo;
    use crate::result::BaseRest;

    /// Verify the caller may list comments under the team.
    pub const fn ensure_user_can_list_infos(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_member(member_info)
    }

    /// Verify the caller may create a comment under the team.
    pub const fn ensure_user_can_create(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_member(member_info)
    }
}

use crate::util::next_snowflake_id;

/// Generate a unique comment identifier backed by a snowflake value.
pub fn gen_id() -> String {
    next_snowflake_id()
}
