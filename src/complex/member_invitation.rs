//! Complex-domain operations for member invitations.

use uuid::Uuid;

use crate::complex::util::{check_user_is_team_admin, check_user_is_team_member};
use crate::part::repo::proxy::ProxyExecute;
use crate::part::repo::step::member::FindByUserTeamId;
use crate::part::repo::step::member_invitation::{
    GetInfoById as MemberInvitationGetInfoById, MemberInvitationStep,
};
use crate::result::{RootError, RootResult};

/// Domain operations for member invitations.
pub struct MemberInvitationComplex;

impl MemberInvitationComplex {
    /// Generate a unique member invitation identifier.
    pub fn gen_id() -> String {
        format!("member_invitation-{}", Uuid::now_v7())
    }

    /// Generate a short invitation code from a unique invitation id.
    pub fn gen_code() -> String {
        let full = Uuid::now_v7().to_string();
        let len = full.len();

        full[len.saturating_sub(6)..].into()
    }
}

/// Permission-gate operations for member invitation entities — invitation-scoped.
pub struct MemberInvitationPermComplex;

impl MemberInvitationPermComplex {
    pub async fn can_user_create<P>(proxy: &mut P, user_id: &str, team_id: &str) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    pub async fn can_user_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_user_is_team_member(proxy, user_id, team_id).await
    }

    pub async fn can_user_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        invitation_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<MemberInvitationGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        let team_id = Self::resolve_team_id(proxy, invitation_id).await?;
        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    pub async fn can_user_delete<P>(
        proxy: &mut P,
        user_id: &str,
        invitation_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<MemberInvitationGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        let team_id = Self::resolve_team_id(proxy, invitation_id).await?;
        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    async fn resolve_team_id<P>(proxy: &mut P, invitation_id: &str) -> RootResult<String>
    where
        P: for<'a> ProxyExecute<MemberInvitationGetInfoById<'a>, Error = RootError>,
    {
        let member_invitation_info = proxy
            .execute(&MemberInvitationStep::get_info_by_id(invitation_id))
            .await?;
        Ok(member_invitation_info.team_id)
    }
}
