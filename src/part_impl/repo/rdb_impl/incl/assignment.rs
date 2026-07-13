use crate::model::assignment::AssignmentInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::model::workset::WorksetInfo;
use crate::part_impl::repo::rdb_impl::incl::{
    self, ChapterByIds, ComicByIds, Incl, TeamByIds, UserByIds, WorksetByIds,
};
use crate::part_impl::shared::RdbConn;
use crate::result::RegularResult;
use crate::value::assignment::AssignmentInclOpt;
use crate::value::incl::expand_incl_opts;

/// Include struct for eager-loading [`ChapterInfo`] data into [`AssignmentInfo`] query results.
struct AssignmentChapterIncl;

impl Incl for AssignmentChapterIncl {
    type Owner = AssignmentInfo;
    type Related = ChapterInfo;
    type Query = ChapterByIds;

    fn resolve_key(assignment_info: &AssignmentInfo) -> Option<&str> {
        Some(&assignment_info.chapter_id)
    }

    fn inject(
        assignment_info: &mut AssignmentInfo,
        chapter_info: Option<ChapterInfo>,
    ) {
        assignment_info.chapter = chapter_info;
    }
}

/// Include struct for eager-loading [`ComicInfo`] data into [`AssignmentInfo`] query results (via chapter).
struct AssignmentChapterComicIncl;

impl Incl for AssignmentChapterComicIncl {
    type Owner = AssignmentInfo;
    type Related = ComicInfo;
    type Query = ComicByIds;

    fn resolve_key(assignment_info: &AssignmentInfo) -> Option<&str> {
        assignment_info
            .chapter
            .as_ref()
            .map(|chapter_info| chapter_info.comic_id.as_str())
    }

    fn inject(
        assignment_info: &mut AssignmentInfo,
        comic_info: Option<ComicInfo>,
    ) {
        //
        let Some(chapter_info) = &mut assignment_info.chapter else {
            return;
        };

        chapter_info.comic = comic_info;
    }
}

/// Include struct for eager-loading [`WorksetInfo`] data into [`AssignmentInfo`] query results (via chapter, comic).
struct AssignmentChapterComicWorksetIncl;

impl Incl for AssignmentChapterComicWorksetIncl {
    type Owner = AssignmentInfo;
    type Related = WorksetInfo;
    type Query = WorksetByIds;

    fn resolve_key(assignment_info: &AssignmentInfo) -> Option<&str> {
        assignment_info
            .chapter
            .as_ref()
            .and_then(|chapter_info| chapter_info.comic.as_ref())
            .map(|comic_info| comic_info.workset_id.as_str())
    }

    fn inject(
        assignment_info: &mut AssignmentInfo,
        workset_info: Option<WorksetInfo>,
    ) {
        //
        let Some(chapter_info) = &mut assignment_info.chapter else {
            return;
        };

        let Some(comic_info) = &mut chapter_info.comic else {
            return;
        };

        comic_info.workset = workset_info;
    }
}

/// Include struct for eager-loading [`TeamInfo`] data into [`AssignmentInfo`] query results (via chapter, comic, workset).
struct AssignmentChapterComicWorksetTeamIncl;

impl Incl for AssignmentChapterComicWorksetTeamIncl {
    type Owner = AssignmentInfo;
    type Related = TeamInfo;
    type Query = TeamByIds;

    fn resolve_key(assignment_info: &AssignmentInfo) -> Option<&str> {
        assignment_info
            .chapter
            .as_ref()
            .and_then(|chapter_info| chapter_info.comic.as_ref())
            .and_then(|comic_info| comic_info.workset.as_ref())
            .map(|workset_info| workset_info.team_id.as_str())
    }

    fn inject(
        assignment_info: &mut AssignmentInfo,
        team_info: Option<TeamInfo>,
    ) {
        //
        let Some(chapter_info) = &mut assignment_info.chapter else {
            return;
        };

        let Some(comic_info) = &mut chapter_info.comic else {
            return;
        };

        comic_info.team = team_info;
    }
}

/// Include struct for eager-loading chapter creator [`UserInfo`] into [`AssignmentInfo`] query results.
struct AssignmentChapterCreatorIncl;

impl Incl for AssignmentChapterCreatorIncl {
    type Owner = AssignmentInfo;
    type Related = UserInfo;
    type Query = UserByIds;

    fn resolve_key(assignment_info: &AssignmentInfo) -> Option<&str> {
        assignment_info
            .chapter
            .as_ref()
            .map(|chapter_info| chapter_info.creator_id.as_str())
    }

    fn inject(
        assignment_info: &mut AssignmentInfo,
        user_info: Option<UserInfo>,
    ) {
        //
        let Some(chapter_info) = &mut assignment_info.chapter else {
            return;
        };

        chapter_info.creator = user_info;
    }
}

/// Include struct for eager-loading comic creator [`UserInfo`] into [`AssignmentInfo`] query results (via chapter).
struct AssignmentChapterComicCreatorIncl;

impl Incl for AssignmentChapterComicCreatorIncl {
    type Owner = AssignmentInfo;
    type Related = UserInfo;
    type Query = UserByIds;

    fn resolve_key(assignment_info: &AssignmentInfo) -> Option<&str> {
        assignment_info
            .chapter
            .as_ref()
            .and_then(|chapter_info| chapter_info.comic.as_ref())
            .map(|comic_info| comic_info.creator_id.as_str())
    }

    fn inject(
        assignment_info: &mut AssignmentInfo,
        user_info: Option<UserInfo>,
    ) {
        //
        let Some(chapter_info) = &mut assignment_info.chapter else {
            return;
        };

        let Some(comic_info) = &mut chapter_info.comic else {
            return;
        };

        comic_info.creator = user_info;
    }
}

/// Include struct for eager-loading [`UserInfo`] data into [`AssignmentInfo`] query results.
struct AssignmentUserIncl;

impl Incl for AssignmentUserIncl {
    type Owner = AssignmentInfo;
    type Related = UserInfo;
    type Query = UserByIds;

    fn resolve_key(assignment_info: &AssignmentInfo) -> Option<&str> {
        Some(&assignment_info.user_id)
    }

    fn inject(
        assignment_info: &mut AssignmentInfo,
        user_info: Option<UserInfo>,
    ) {
        assignment_info.user = user_info;
    }
}

/// Populates assignment query results with eagerly-loaded related entity data.
pub async fn populate_assignment_incls(
    conn: &mut RdbConn,
    infos: &mut [AssignmentInfo],
    incl_opt: &[AssignmentInclOpt],
) -> RegularResult<()> {
    //
    for incl_opt in expand_incl_opts(incl_opt) {
        match incl_opt {
            //
            AssignmentInclOpt::User => {
                incl::populate::<AssignmentUserIncl>(conn, infos).await?
            }

            AssignmentInclOpt::Chapter => {
                incl::populate::<AssignmentChapterIncl>(conn, infos).await?
            }

            AssignmentInclOpt::ChapterComic => {
                incl::populate::<AssignmentChapterComicIncl>(conn, infos)
                    .await?
            }

            AssignmentInclOpt::ChapterComicWorkset => {
                incl::populate::<AssignmentChapterComicWorksetIncl>(conn, infos)
                    .await?
            }

            AssignmentInclOpt::ChapterComicWorksetTeam => {
                incl::populate::<AssignmentChapterComicWorksetTeamIncl>(
                    conn, infos,
                )
                .await?
            }

            AssignmentInclOpt::ChapterCreator => {
                incl::populate::<AssignmentChapterCreatorIncl>(conn, infos)
                    .await?
            }

            AssignmentInclOpt::ChapterComicCreator => {
                incl::populate::<AssignmentChapterComicCreatorIncl>(conn, infos)
                    .await?
            }
        }
    }

    Ok(())
}
