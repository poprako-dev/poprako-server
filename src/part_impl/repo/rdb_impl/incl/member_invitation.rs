use async_trait::async_trait;

use crate::model::member_invitation::MemberInvitationInfo;
use crate::model::user::UserInfo;
use crate::part_impl::shared::RdbConn;
use crate::part_impl::repo::rdb_impl::incl::{self, Incl, UserByIds};
use crate::result::RegularResult;
use crate::value::member_invitation::MemberInvitationInclOpt;

struct MemberInvitationInvitorIncl;

#[async_trait]
impl Incl for MemberInvitationInvitorIncl {
    type Owner = MemberInvitationInfo;
    type Related = UserInfo;
    type Query = UserByIds;

    fn resolve_key(owner: &MemberInvitationInfo) -> Option<&str> {
        Some(&owner.invitor_id)
    }

    fn inject(owner: &mut MemberInvitationInfo, related: Option<UserInfo>) {
        owner.invitor = related;
    }
}

pub async fn populate_member_invitation_incls(
    conn: &mut RdbConn,
    infos: &mut [MemberInvitationInfo],
    incl_opt: &[MemberInvitationInclOpt],
) -> RegularResult<()> {
    if incl_opt.contains(&MemberInvitationInclOpt::Invitor) {
        incl::populate::<MemberInvitationInvitorIncl>(conn, infos).await?;
    }
    Ok(())
}
