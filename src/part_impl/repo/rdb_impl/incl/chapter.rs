use async_trait::async_trait;

use crate::model::chapter_model;
use crate::model::comic_model;
use crate::model::team_model;
use crate::model::user_model;
use crate::model::workset_model;
use crate::part_impl::repo::rdb_impl::incl::{
    self, ComicByIds, Incl, TeamByIds, UserByIds, WorksetByIds,
};
use crate::part_impl::shared::RdbConn;
use crate::result::RegularResult;
use crate::value::chapter::ChapterInclOpt;
use crate::value::incl::expand_incl_opts;

/// Include struct for eager-loading [`ComicInfo`] data into [`ChapterInfo`] query results.
struct ChapterComicIncl;

#[async_trait]
impl Incl for ChapterComicIncl {
    type Owner = chapter_model::Info;
    type Related = comic_model::Info;
    type Query = ComicByIds;

    fn resolve_key(chapter_info: &chapter_model::Info) -> Option<&str> {
        Some(&chapter_info.comic_id)
    }

    fn inject(
        chapter_info: &mut chapter_model::Info,
        comic_info: Option<comic_model::Info>,
    ) {
        chapter_info.comic = comic_info;
    }
}

/// Include struct for eager-loading [`WorksetInfo`] data into [`ChapterInfo`] query results (via comic).
struct ChapterComicWorksetIncl;

#[async_trait]
impl Incl for ChapterComicWorksetIncl {
    type Owner = chapter_model::Info;
    type Related = workset_model::Info;
    type Query = WorksetByIds;

    fn resolve_key(chapter_info: &chapter_model::Info) -> Option<&str> {
        chapter_info
            .comic
            .as_ref()
            .map(|comic_info| comic_info.workset_id.as_str())
    }

    fn inject(
        chapter_info: &mut chapter_model::Info,
        workset_info: Option<workset_model::Info>,
    ) {
        //
        let Some(comic_info) = &mut chapter_info.comic else {
            return;
        };

        comic_info.workset = workset_info;
    }
}

/// Include struct for eager-loading [`TeamInfo`] data into [`ChapterInfo`] query results (via comic, workset).
struct ChapterComicWorksetTeamIncl;

#[async_trait]
impl Incl for ChapterComicWorksetTeamIncl {
    type Owner = chapter_model::Info;
    type Related = team_model::Info;
    type Query = TeamByIds;

    fn resolve_key(chapter_info: &chapter_model::Info) -> Option<&str> {
        chapter_info
            .comic
            .as_ref()
            .and_then(|comic_info| comic_info.workset.as_ref())
            .map(|workset_info| workset_info.team_id.as_str())
    }

    fn inject(
        chapter_info: &mut chapter_model::Info,
        team_info: Option<team_model::Info>,
    ) {
        //
        let Some(comic_info) = &mut chapter_info.comic else {
            return;
        };

        comic_info.team = team_info;
    }
}

/// Include struct for eager-loading comic creator [`UserInfo`] into [`ChapterInfo`] query results.
struct ChapterComicCreatorIncl;

#[async_trait]
impl Incl for ChapterComicCreatorIncl {
    type Owner = chapter_model::Info;
    type Related = user_model::Info;
    type Query = UserByIds;

    fn resolve_key(chapter_info: &chapter_model::Info) -> Option<&str> {
        chapter_info
            .comic
            .as_ref()
            .map(|comic_info| comic_info.creator_id.as_str())
    }

    fn inject(
        chapter_info: &mut chapter_model::Info,
        user_info: Option<user_model::Info>,
    ) {
        //
        let Some(comic_info) = &mut chapter_info.comic else {
            return;
        };

        comic_info.creator = user_info;
    }
}

/// Include struct for eager-loading creator [`UserInfo`] into [`ChapterInfo`] query results.
struct ChapterCreatorIncl;

#[async_trait]
impl Incl for ChapterCreatorIncl {
    type Owner = chapter_model::Info;
    type Related = user_model::Info;
    type Query = UserByIds;

    fn resolve_key(chapter_info: &chapter_model::Info) -> Option<&str> {
        Some(&chapter_info.creator_id)
    }

    fn inject(
        chapter_info: &mut chapter_model::Info,
        user_info: Option<user_model::Info>,
    ) {
        chapter_info.creator = user_info;
    }
}

/// Populates chapter query results with eagerly-loaded related entity data.
pub async fn populate_chapter_incls(
    conn: &mut RdbConn,
    infos: &mut [chapter_model::Info],
    incl_opt: &[ChapterInclOpt],
) -> RegularResult<()> {
    //
    for incl_opt in expand_incl_opts(incl_opt) {
        match incl_opt {
            //
            ChapterInclOpt::Comic => {
                incl::populate::<ChapterComicIncl>(conn, infos).await?
            }

            ChapterInclOpt::ComicWorkset => {
                incl::populate::<ChapterComicWorksetIncl>(conn, infos).await?
            }

            ChapterInclOpt::ComicWorksetTeam => {
                incl::populate::<ChapterComicWorksetTeamIncl>(conn, infos)
                    .await?
            }

            ChapterInclOpt::ComicCreator => {
                incl::populate::<ChapterComicCreatorIncl>(conn, infos).await?
            }

            ChapterInclOpt::Creator => {
                incl::populate::<ChapterCreatorIncl>(conn, infos).await?
            }
        }
    }

    Ok(())
}
