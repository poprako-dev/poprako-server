use tracing::instrument;

use crate::model::announcement::AnnouncementInfo;
use crate::model::user::UserInfo;
use crate::part_impl::repo::rdb_impl::incl::{self, Incl, UserByIds};
use crate::part_impl::shared::RdbConn;
use crate::result::{BaseResult, accept};
use crate::value::announcement::AnnouncementInclOpt;

/// Include struct for eager-loading [`UserInfo`] data into [`AnnouncementInfo`] query results.
struct AnnouncementUserIncl;

impl Incl for AnnouncementUserIncl {
    type Owner = AnnouncementInfo;
    type Related = UserInfo;
    type Query = UserByIds;

    fn resolve_key(owner: &AnnouncementInfo) -> Option<&str> {
        Some(&owner.user_id)
    }

    fn inject(owner: &mut AnnouncementInfo, related: Option<UserInfo>) {
        owner.user = related;
    }
}

/// Populates announcement query results with eagerly-loaded user data.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn populate_announcement_incls(
    conn: &mut RdbConn,
    infos: &mut [AnnouncementInfo],
    incl_opt: &[AnnouncementInclOpt],
) -> BaseResult<()> {
    //
    if incl_opt.contains(&AnnouncementInclOpt::User) {
        incl::populate::<AnnouncementUserIncl>(conn, infos).await?;
    }

    accept(())
}
