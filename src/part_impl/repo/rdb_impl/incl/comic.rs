use async_trait::async_trait;

use crate::model::comic::ComicInfo;
use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::model::workset::WorksetInfo;
use crate::part_impl::repo::rdb_impl::incl::{
    self, Incl, TeamByIds, UserByIds, WorksetByIds,
};
use crate::part_impl::shared::RdbConn;
use crate::result::RegularResult;
use crate::value::comic::ComicInclOpt;
use crate::value::incl::expand_incl_opts;

/// Include struct for eager-loading [`WorksetInfo`] data into [`ComicInfo`] query results.
struct ComicWorksetIncl;

#[async_trait]
impl Incl for ComicWorksetIncl {
    type Owner = ComicInfo;
    type Related = WorksetInfo;
    type Query = WorksetByIds;

    fn resolve_key(comic_info: &ComicInfo) -> Option<&str> {
        Some(&comic_info.workset_id)
    }

    fn inject(comic_info: &mut ComicInfo, workset_info: Option<WorksetInfo>) {
        comic_info.workset = workset_info;
    }
}

/// Include struct for eager-loading [`TeamInfo`] data into [`ComicInfo`] query results (via workset).
struct ComicWorksetTeamIncl;

#[async_trait]
impl Incl for ComicWorksetTeamIncl {
    type Owner = ComicInfo;
    type Related = TeamInfo;
    type Query = TeamByIds;

    fn resolve_key(comic_info: &ComicInfo) -> Option<&str> {
        comic_info
            .workset
            .as_ref()
            .map(|workset_info| workset_info.team_id.as_str())
    }

    fn inject(comic_info: &mut ComicInfo, team_info: Option<TeamInfo>) {
        comic_info.team = team_info;
    }
}

/// Include struct for eager-loading creator [`UserInfo`] into [`ComicInfo`] query results.
struct ComicCreatorIncl;

#[async_trait]
impl Incl for ComicCreatorIncl {
    type Owner = ComicInfo;
    type Related = UserInfo;
    type Query = UserByIds;

    fn resolve_key(comic_info: &ComicInfo) -> Option<&str> {
        Some(&comic_info.creator_id)
    }

    fn inject(comic_info: &mut ComicInfo, user_info: Option<UserInfo>) {
        comic_info.creator = user_info;
    }
}

/// Populates comic query results with eagerly-loaded related entity data.
pub async fn populate_comic_incls(
    conn: &mut RdbConn,
    infos: &mut [ComicInfo],
    incl_opt: &[ComicInclOpt],
) -> RegularResult<()> {
    for incl_opt in expand_incl_opts(incl_opt) {
        match incl_opt {
            ComicInclOpt::Workset => {
                incl::populate::<ComicWorksetIncl>(conn, infos).await?
            }
            ComicInclOpt::WorksetTeam => {
                incl::populate::<ComicWorksetTeamIncl>(conn, infos).await?
            }
            ComicInclOpt::Creator => {
                incl::populate::<ComicCreatorIncl>(conn, infos).await?
            }
        }
    }
    Ok(())
}
