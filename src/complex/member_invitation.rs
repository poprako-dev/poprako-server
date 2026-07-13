//! Complex-domain opers for member invitations.

use poprako_orchestra::Proxy;

use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_member,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::member_invitation::GetMemberInvitationInfo;
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
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
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
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
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
        P: for<'a, 'b> Proxy<
                GetMemberInvitationInfo<'a, 'b>,
                Error = RegularError,
            > + for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
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
        P: for<'a, 'b> Proxy<
                GetMemberInvitationInfo<'a, 'b>,
                Error = RegularError,
            > + for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
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
        P: for<'a, 'b> Proxy<
                GetMemberInvitationInfo<'a, 'b>,
                Error = RegularError,
            >,
    {
        let get_member_invitation_info = GetMemberInvitationInfo::Id {
            id: invitation_id,
            incls: &[],
        };

        let member_invitation_info =
            proxy.exec(&get_member_invitation_info).await?;

        Ok(member_invitation_info.team_id)
    }
}
