use crate::model::member_invitation::MemberInvitationInfo;
use crate::model::user::UserInfo;
use crate::part_impl::repo::rdb_impl::incl::{self, Incl, UserByIds};
use crate::part_impl::shared::RdbConn;
use crate::result::RegularResult;
use crate::value::member_invitation::MemberInvitationInclOpt;

/// Include struct for eager-loading invitor [`UserInfo`] into [`MemberInvitationInfo`] query results.
struct MemberInvitationInvitorIncl;

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

/// Populates member invitation query results with eagerly-loaded invitor user data.
pub async fn populate_member_invitation_incls(
    conn: &mut RdbConn,
    infos: &mut [MemberInvitationInfo],
    incl_opt: &[MemberInvitationInclOpt],
) -> RegularResult<()> {
    //
    if incl_opt.contains(&MemberInvitationInclOpt::Invitor) {
        incl::populate::<MemberInvitationInvitorIncl>(conn, infos).await?;
    }

    Ok(())
}
