//! Complex-domain opers for team comments.
use crate::complex::util::check_user_is_team_member;
use crate::model::read::proj::member::MemberInfo;
use crate::result::BaseRest;
use crate::util::next_snowflake_id;

/// Domain opers for comments.
pub struct CommentComplex;

impl CommentComplex {
    /// Generate a unique comment identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }
}

/// Permission-gate opers for comments.
pub struct CommentPermComplex;

impl CommentPermComplex {
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
