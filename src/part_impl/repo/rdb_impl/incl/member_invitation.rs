use async_trait::async_trait;

use crate::model::member_invitation_model;
use crate::model::user_model;
use crate::part_impl::repo::rdb_impl::incl::{self, Incl, UserByIds};
use crate::part_impl::shared::RdbConn;
use crate::result::RegularResult;
use crate::value::member_invitation::MemberInvitationInclOpt;

/// Include struct for eager-loading invitor [`UserInfo`] into [`MemberInvitationInfo`] query results.
struct MemberInvitationInvitorIncl;

#[async_trait]
impl Incl for MemberInvitationInvitorIncl {
    type Owner = member_invitation_model::Info;
    type Related = user_model::Info;
    type Query = UserByIds;

    fn resolve_key(owner: &member_invitation_model::Info) -> Option<&str> {
        Some(&owner.invitor_id)
    }

    fn inject(
        owner: &mut member_invitation_model::Info,
        related: Option<user_model::Info>,
    ) {
        owner.invitor = related;
    }
}

/// Populates member invitation query results with eagerly-loaded invitor user data.
pub async fn populate_member_invitation_incls(
    conn: &mut RdbConn,
    infos: &mut [member_invitation_model::Info],
    incl_opt: &[MemberInvitationInclOpt],
) -> RegularResult<()> {
    //
    if incl_opt.contains(&MemberInvitationInclOpt::Invitor) {
        incl::populate::<MemberInvitationInvitorIncl>(conn, infos).await?;
    }

    Ok(())
}
