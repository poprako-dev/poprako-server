use async_trait::async_trait;

use crate::model::announcement_model;
use crate::model::user_model;
use crate::part_impl::repo::rdb_impl::incl::{self, Incl, UserByIds};
use crate::part_impl::shared::RdbConn;
use crate::result::RegularResult;
use crate::value::announcement::AnnouncementInclOpt;

/// Include struct for eager-loading [`UserInfo`] data into [`AnnouncementInfo`] query results.
struct AnnouncementUserIncl;

#[async_trait]
impl Incl for AnnouncementUserIncl {
    type Owner = announcement_model::Info;
    type Related = user_model::Info;
    type Query = UserByIds;

    fn resolve_key(owner: &announcement_model::Info) -> Option<&str> {
        Some(&owner.user_id)
    }

    fn inject(
        owner: &mut announcement_model::Info,
        related: Option<user_model::Info>,
    ) {
        owner.user = related;
    }
}

/// Populates announcement query results with eagerly-loaded user data.
pub async fn populate_announcement_incls(
    conn: &mut RdbConn,
    infos: &mut [announcement_model::Info],
    incl_opt: &[AnnouncementInclOpt],
) -> RegularResult<()> {
    //
    if incl_opt.contains(&AnnouncementInclOpt::User) {
        incl::populate::<AnnouncementUserIncl>(conn, infos).await?;
    }

    Ok(())
}
