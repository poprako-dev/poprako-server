use async_trait::async_trait;

use crate::model::{comic_model, team_model, user_model, workset_model};
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
    type Owner = comic_model::Info;
    type Related = workset_model::Info;
    type Query = WorksetByIds;

    fn resolve_key(comic_info: &comic_model::Info) -> Option<&str> {
        Some(&comic_info.workset_id)
    }

    fn inject(
        comic_info: &mut comic_model::Info,
        workset_info: Option<workset_model::Info>,
    ) {
        comic_info.workset = workset_info;
    }
}

/// Include struct for eager-loading [`TeamInfo`] data into [`ComicInfo`] query results (via workset).
struct ComicWorksetTeamIncl;

#[async_trait]
impl Incl for ComicWorksetTeamIncl {
    type Owner = comic_model::Info;
    type Related = team_model::Info;
    type Query = TeamByIds;

    fn resolve_key(comic_info: &comic_model::Info) -> Option<&str> {
        comic_info
            .workset
            .as_ref()
            .map(|workset_info| workset_info.team_id.as_str())
    }

    fn inject(
        comic_info: &mut comic_model::Info,
        team_info: Option<team_model::Info>,
    ) {
        comic_info.team = team_info;
    }
}

/// Include struct for eager-loading creator [`UserInfo`] into [`ComicInfo`] query results.
struct ComicCreatorIncl;

#[async_trait]
impl Incl for ComicCreatorIncl {
    type Owner = comic_model::Info;
    type Related = user_model::Info;
    type Query = UserByIds;

    fn resolve_key(comic_info: &comic_model::Info) -> Option<&str> {
        Some(&comic_info.creator_id)
    }

    fn inject(
        comic_info: &mut comic_model::Info,
        user_info: Option<user_model::Info>,
    ) {
        comic_info.creator = user_info;
    }
}

/// Populates comic query results with eagerly-loaded related entity data.
pub async fn populate_comic_incls(
    conn: &mut RdbConn,
    infos: &mut [comic_model::Info],
    incl_opt: &[ComicInclOpt],
) -> RegularResult<()> {
    //
    for incl_opt in expand_incl_opts(incl_opt) {
        match incl_opt {
            //
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
