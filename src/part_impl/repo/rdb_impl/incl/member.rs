use async_trait::async_trait;

use crate::model::member_model;
use crate::model::team_model;
use crate::model::user_model;
use crate::part_impl::repo::rdb_impl::incl::{
    self, Incl, TeamByIds, UserByIds,
};
use crate::part_impl::shared::RdbConn;
use crate::result::RegularResult;
use crate::value::member::MemberInclOpt;

/// Include struct for eager-loading [`UserInfo`] data into [`MemberInfo`] query results.
struct MemberUserIncl;

#[async_trait]
impl Incl for MemberUserIncl {
    type Owner = member_model::Info;
    type Related = user_model::Info;
    type Query = UserByIds;

    fn resolve_key(owner: &member_model::Info) -> Option<&str> {
        Some(&owner.user_id)
    }

    fn inject(
        owner: &mut member_model::Info,
        related: Option<user_model::Info>,
    ) {
        owner.user = related;
    }
}

/// Include struct for eager-loading [`TeamInfo`] data into [`MemberInfo`] query results.
struct MemberTeamIncl;

#[async_trait]
impl Incl for MemberTeamIncl {
    type Owner = member_model::Info;
    type Related = team_model::Info;
    type Query = TeamByIds;

    fn resolve_key(owner: &member_model::Info) -> Option<&str> {
        Some(&owner.team_id)
    }

    fn inject(
        owner: &mut member_model::Info,
        related: Option<team_model::Info>,
    ) {
        owner.team = related;
    }
}

/// Populates member query results with eagerly-loaded user and team data.
pub async fn populate_member_incls(
    conn: &mut RdbConn,
    infos: &mut [member_model::Info],
    incl_opt: &[MemberInclOpt],
) -> RegularResult<()> {
    //
    if incl_opt.contains(&MemberInclOpt::User) {
        incl::populate::<MemberUserIncl>(conn, infos).await?;
    }

    if incl_opt.contains(&MemberInclOpt::Team) {
        incl::populate::<MemberTeamIncl>(conn, infos).await?;
    }

    Ok(())
}
