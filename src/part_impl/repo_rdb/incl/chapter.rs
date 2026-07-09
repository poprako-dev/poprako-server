use async_trait::async_trait;

use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::model::workset::WorksetInfo;
use crate::part_impl::rdb_core::RdbConn;
use crate::part_impl::repo_rdb::incl::{
    self, ComicByIds, Incl, TeamByIds, UserByIds, WorksetByIds,
};
use crate::result::RegularResult;
use crate::value::chapter::ChapterInclOpt;
use crate::value::incl::expand_incl_opts;

struct ChapterComicIncl;

#[async_trait]
impl Incl for ChapterComicIncl {
    type Owner = ChapterInfo;
    type Related = ComicInfo;
    type Query = ComicByIds;

    fn resolve_key(chapter_info: &ChapterInfo) -> Option<&str> {
        Some(&chapter_info.comic_id)
    }

    fn inject(chapter_info: &mut ChapterInfo, comic_info: Option<ComicInfo>) {
        chapter_info.comic = comic_info;
    }
}

struct ChapterComicWorksetIncl;

#[async_trait]
impl Incl for ChapterComicWorksetIncl {
    type Owner = ChapterInfo;
    type Related = WorksetInfo;
    type Query = WorksetByIds;

    fn resolve_key(chapter_info: &ChapterInfo) -> Option<&str> {
        chapter_info
            .comic
            .as_ref()
            .map(|comic_info| comic_info.workset_id.as_str())
    }

    fn inject(
        chapter_info: &mut ChapterInfo,
        workset_info: Option<WorksetInfo>,
    ) {
        let Some(comic_info) = &mut chapter_info.comic else {
            return;
        };
        comic_info.workset = workset_info;
    }
}

struct ChapterComicWorksetTeamIncl;

#[async_trait]
impl Incl for ChapterComicWorksetTeamIncl {
    type Owner = ChapterInfo;
    type Related = TeamInfo;
    type Query = TeamByIds;

    fn resolve_key(chapter_info: &ChapterInfo) -> Option<&str> {
        chapter_info
            .comic
            .as_ref()
            .and_then(|comic_info| comic_info.workset.as_ref())
            .map(|workset_info| workset_info.team_id.as_str())
    }

    fn inject(chapter_info: &mut ChapterInfo, team_info: Option<TeamInfo>) {
        let Some(comic_info) = &mut chapter_info.comic else {
            return;
        };
        comic_info.team = team_info;
    }
}

struct ChapterComicCreatorIncl;

#[async_trait]
impl Incl for ChapterComicCreatorIncl {
    type Owner = ChapterInfo;
    type Related = UserInfo;
    type Query = UserByIds;

    fn resolve_key(chapter_info: &ChapterInfo) -> Option<&str> {
        chapter_info
            .comic
            .as_ref()
            .map(|comic_info| comic_info.creator_id.as_str())
    }

    fn inject(chapter_info: &mut ChapterInfo, user_info: Option<UserInfo>) {
        let Some(comic_info) = &mut chapter_info.comic else {
            return;
        };
        comic_info.creator = user_info;
    }
}

struct ChapterCreatorIncl;

#[async_trait]
impl Incl for ChapterCreatorIncl {
    type Owner = ChapterInfo;
    type Related = UserInfo;
    type Query = UserByIds;

    fn resolve_key(chapter_info: &ChapterInfo) -> Option<&str> {
        Some(&chapter_info.creator_id)
    }

    fn inject(chapter_info: &mut ChapterInfo, user_info: Option<UserInfo>) {
        chapter_info.creator = user_info;
    }
}

pub async fn populate_chapter_incls(
    conn: &mut RdbConn,
    infos: &mut [ChapterInfo],
    incl_opt: &[ChapterInclOpt],
) -> RegularResult<()> {
    for incl_opt in expand_incl_opts(incl_opt) {
        match incl_opt {
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
