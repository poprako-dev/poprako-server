//! Complex-domain opers for member invitations.

use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_member,
};
use crate::model::read::proj::member::MemberInfo;
use crate::result::BaseRest;
use crate::util::next_snowflake_id;

/// Domain opers for member invitations.
pub struct MemberInvitationComplex;

impl MemberInvitationComplex {
    /// Generate a unique member invitation identifier.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Generate a short invitation code from a unique invitation id.
    pub fn gen_code() -> String {
        //
        let full = next_snowflake_id();

        let skipped_count = full.chars().count().saturating_sub(6);

        full.chars().skip(skipped_count).collect()
    }
}

/// Permission-gate opers for member invitation entities — invitation-scoped.
pub struct MemberInvitationPermComplex;

impl MemberInvitationPermComplex {
    /// Verify the caller is a team admin and may create invitations for the team.
    pub fn ensure_user_can_create(member_info: &MemberInfo) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }

    /// Verify the caller is a team member and may list invitations for the team.
    pub const fn ensure_user_can_list_infos(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_member(member_info)
    }

    /// Verify the caller is a team admin of the invitation's owning team.
    pub fn ensure_user_can_update_info(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }

    /// Verify the caller is a team admin and may delete the invitation.
    pub fn ensure_user_can_delete(member_info: &MemberInfo) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }
}
