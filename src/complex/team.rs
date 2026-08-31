//! Pure rules for team entities.

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_member,
};
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::user::UserInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;

/// Pure domain operations for team entities.
pub struct TeamComplex;

impl TeamComplex {
    /// Generate a unique, time-ordered team identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }
}

/// Pure permission rules for team entities.
pub struct TeamPermComplex;

impl TeamPermComplex {
    /// Verify the caller may mark themselves online in the target team.
    pub const fn ensure_user_can_mark_self_online(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_member(member_info)
    }

    /// Verify the caller may list online users in the target team.
    pub const fn ensure_user_can_list_online_user_ids(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_member(member_info)
    }

    /// Verify the caller may update team information.
    pub fn ensure_user_can_update_info(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }

    /// Verify the caller may allocate a team avatar.
    pub fn ensure_user_can_alloc_avatar(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }

    /// Verify the caller may confirm a team avatar upload.
    pub fn ensure_user_can_mark_avatar_uploaded(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }

    /// Verify the caller may delete a team.
    pub fn ensure_user_can_delete(member_info: &MemberInfo) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }

    /// Verify the user can create a team.
    pub fn ensure_user_can_create(user_info: &UserInfo) -> BaseRest<()> {
        Self::check_user_is_sadmin(user_info)
    }

    /// Verify the user can list team infos.
    pub fn ensure_user_can_list_infos(user_info: &UserInfo) -> BaseRest<()> {
        Self::check_user_is_sadmin(user_info)
    }

    // Verify that loaded user evidence belongs to a super-admin.
    fn check_user_is_sadmin(user_info: &UserInfo) -> BaseRest<()> {
        //
        if !user_info.is_sadmin {
            //
            let err_message = trl("error-sadmin-required");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Perm,
                err_message = %err_message,
                user_id = %user_info.id,
                is_sadmin = user_info.is_sadmin,
                "expected error: super-admin perm required",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                message: err_message,
            });
        }

        accept(())
    }
}
