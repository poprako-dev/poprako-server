use crate::model::assignment::AssignmentInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::model::workset::WorksetInfo;
use crate::part_impl::repo::mock_impl::MockState;
use crate::value::assignment::AssignmentInclOpt;
use crate::value::incl::expand_incl_opts;

fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    state.users.iter().find(|info| info.id == user_id).cloned()
}

fn find_chapter(state: &MockState, id: &str) -> Option<ChapterInfo> {
    //
    let mut chapter_info = state
        .chapters
        .iter()
        .find(|chapter_info| chapter_info.id == id)
        .cloned()?;

    chapter_info.comic = None;

    chapter_info.creator = None;

    Some(chapter_info)
}

fn find_comic(state: &MockState, id: &str) -> Option<ComicInfo> {
    //
    let mut comic_info = state
        .comics
        .iter()
        .find(|comic_info| comic_info.id == id)
        .cloned()?;

    comic_info.workset = None;

    comic_info.team = None;

    comic_info.creator = None;

    Some(comic_info)
}

fn find_workset(state: &MockState, id: &str) -> Option<WorksetInfo> {
    state.worksets.iter().find(|info| info.id == id).cloned()
}

fn find_team(state: &MockState, workset: &WorksetInfo) -> Option<TeamInfo> {
    state
        .teams
        .iter()
        .find(|info| info.id == workset.team_id)
        .cloned()
}

/// Applies the requested include options to an [`AssignmentInfo`], resolving
/// user, chapter, comic, workset, team, and creator relations from the mock state.
pub fn apply_assignment_incls(
    state: &MockState,
    assignment: &mut AssignmentInfo,
    incls: &[AssignmentInclOpt],
) {
    //
    assignment.user = None;

    assignment.chapter = None;

    for incl in expand_incl_opts(incls) {
        match incl {
            //
            AssignmentInclOpt::User => {
                assignment.user = find_user(state, &assignment.user_id);
            }

            AssignmentInclOpt::Chapter => {
                assignment.chapter =
                    find_chapter(state, &assignment.chapter_id);
            }

            AssignmentInclOpt::ChapterComic => {
                //
                let Some(chapter) = &mut assignment.chapter else {
                    continue;
                };

                chapter.comic = find_comic(state, &chapter.comic_id);
            }

            AssignmentInclOpt::ChapterComicWorkset => {
                //
                let Some(comic) =
                    assignment.chapter.as_mut().and_then(|v| v.comic.as_mut())
                else {
                    continue;
                };

                comic.workset = find_workset(state, &comic.workset_id);
            }

            AssignmentInclOpt::ChapterComicWorksetTeam => {
                //
                let Some(comic) =
                    assignment.chapter.as_mut().and_then(|v| v.comic.as_mut())
                else {
                    continue;
                };

                let Some(workset) = &comic.workset else {
                    continue;
                };

                comic.team = find_team(state, workset);
            }

            AssignmentInclOpt::ChapterCreator => {
                //
                let Some(chapter) = &mut assignment.chapter else {
                    continue;
                };

                chapter.creator = find_user(state, &chapter.creator_id);
            }

            AssignmentInclOpt::ChapterComicCreator => {
                //
                let Some(comic) =
                    assignment.chapter.as_mut().and_then(|v| v.comic.as_mut())
                else {
                    continue;
                };

                comic.creator = find_user(state, &comic.creator_id);
            }
        }
    }
}
