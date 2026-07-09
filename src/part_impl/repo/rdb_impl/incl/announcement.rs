use async_trait::async_trait;

use crate::model::announcement::AnnouncementInfo;
use crate::model::user::UserInfo;
use crate::part_impl::shared::RdbConn;
use crate::part_impl::repo::rdb_impl::incl::{self, Incl, UserByIds};
use crate::result::RegularResult;
use crate::value::announcement::AnnouncementInclOpt;

struct AnnouncementUserIncl;

#[async_trait]
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

pub async fn populate_announcement_incls(
    conn: &mut RdbConn,
    infos: &mut [AnnouncementInfo],
    incl_opt: &[AnnouncementInclOpt],
) -> RegularResult<()> {
    if incl_opt.contains(&AnnouncementInclOpt::User) {
        incl::populate::<AnnouncementUserIncl>(conn, infos).await?;
    }
    Ok(())
}
