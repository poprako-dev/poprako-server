use async_trait::async_trait;

use crate::model::member::MemberInfo;
use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::part_impl::repo_rdb::incl::{self, Incl, TeamByIds, UserByIds};
use crate::part_impl::shared_rdb::RdbConn;
use crate::result::RegularResult;
use crate::value::member::MemberInclOpt;

struct MemberUserIncl;

#[async_trait]
impl Incl for MemberUserIncl {
    type Owner = MemberInfo;
    type Related = UserInfo;
    type Query = UserByIds;

    fn resolve_key(owner: &MemberInfo) -> Option<&str> {
        Some(&owner.user_id)
    }

    fn inject(owner: &mut MemberInfo, related: Option<UserInfo>) {
        owner.user = related;
    }
}

struct MemberTeamIncl;

#[async_trait]
impl Incl for MemberTeamIncl {
    type Owner = MemberInfo;
    type Related = TeamInfo;
    type Query = TeamByIds;

    fn resolve_key(owner: &MemberInfo) -> Option<&str> {
        Some(&owner.team_id)
    }

    fn inject(owner: &mut MemberInfo, related: Option<TeamInfo>) {
        owner.team = related;
    }
}

pub async fn populate_member_incls(
    conn: &mut RdbConn,
    infos: &mut [MemberInfo],
    incl_opt: &[MemberInclOpt],
) -> RegularResult<()> {
    if incl_opt.contains(&MemberInclOpt::User) {
        incl::populate::<MemberUserIncl>(conn, infos).await?;
    }

    if incl_opt.contains(&MemberInclOpt::Team) {
        incl::populate::<MemberTeamIncl>(conn, infos).await?;
    }

    Ok(())
}
