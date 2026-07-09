//! Complex-domain opers for member invitations.

use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_member,
};
use crate::part::repo::step::member::FindInfoByUserIdAndTeamId;
use crate::part::repo::step::member_invitation::{
    GetInfoById as MemberInvitationGetInfoById, MemberInvitationStep,
};
use crate::part::shared::proxy::ProxyExecute;
use crate::result::{RegularError, RegularResult};
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

        let len = full.len();

        full[len.saturating_sub(6)..].into()
    }
}

/// Permission-gate opers for member invitation entities — invitation-scoped.
pub struct MemberInvitationPermComplex;

impl MemberInvitationPermComplex {
    /// Verify the caller is a team admin and may create invitations for the team.
    pub async fn can_user_create<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<
                FindInfoByUserIdAndTeamId<'a>,
                Error = RegularError,
            >,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team member and may list invitations for the team.
    pub async fn can_user_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<
                FindInfoByUserIdAndTeamId<'a>,
                Error = RegularError,
            >,
    {
        check_user_is_team_member(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team admin of the invitation's owning team.
    pub async fn can_user_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        invitation_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<
                MemberInvitationGetInfoById<'a>,
                Error = RegularError,
            > + for<'a> ProxyExecute<
                FindInfoByUserIdAndTeamId<'a>,
                Error = RegularError,
            >,
    {
        let team_id = Self::resolve_team_id(proxy, invitation_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team admin and may delete the invitation.
    pub async fn can_user_delete<P>(
        proxy: &mut P,
        user_id: &str,
        invitation_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<
                MemberInvitationGetInfoById<'a>,
                Error = RegularError,
            > + for<'a> ProxyExecute<
                FindInfoByUserIdAndTeamId<'a>,
                Error = RegularError,
            >,
    {
        let team_id = Self::resolve_team_id(proxy, invitation_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    /// Resolve the owning team ID from an invitation ID.
    async fn resolve_team_id<P>(
        proxy: &mut P,
        invitation_id: &str,
    ) -> RegularResult<String>
    where
        P: for<'a> ProxyExecute<
                MemberInvitationGetInfoById<'a>,
                Error = RegularError,
            >,
    {
        let member_invitation_info = proxy
            .execute(&MemberInvitationStep::get_info_by_id(invitation_id, &[]))
            .await?;

        Ok(member_invitation_info.team_id)
    }
}
