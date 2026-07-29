use crate::model::assignment::AssignmentInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::model::workset::WorksetInfo;
use crate::part_impl::repo::mock_impl::MockState;
use crate::value::assignment::AssignmentInclOpt;
use crate::value::incl::expand_incl_opts;

/// Applies the requested include options to an [`AssignmentInfo`], resolving
/// user, chapter, comic, workset, team, and creator relations from the mock state.
pub fn apply_assignment_incls(
    state: &MockState,
    assignment: &mut AssignmentInfo,
    incls: &[AssignmentInclOpt],
) {
    //
    // Fill all optional relations from empty state, then enrich requested fields.
    assignment.user = None;

    assignment.chapter = None;

    for incl in expand_incl_opts(incls) {
        match incl {
            //
            // Populate user from top-level relation.
            AssignmentInclOpt::User => {
                assignment.user = find_user(state, &assignment.user_id);
            }

            // Populate chapter without nested fields set.
            AssignmentInclOpt::Chapter => {
                assignment.chapter =
                    find_chapter(state, &assignment.chapter_id);
            }

            // Populate chapter comic, then continue include expansion on it.
            AssignmentInclOpt::ChapterComic => {
                //
                // This include intentionally keeps comic nested and resolves it lazily.
                let Some(chapter) = &mut assignment.chapter else {
                    continue;
                };

                chapter.comic = find_comic(state, &chapter.comic_id);
            }

            // Populate comic workspace to expose workset filters and ordering.
            AssignmentInclOpt::ChapterComicWorkset => {
                //
                // Only resolve workset after comic is available.
                let Some(comic) =
                    assignment.chapter.as_mut().and_then(|v| v.comic.as_mut())
                else {
                    continue;
                };

                comic.workset = find_workset(state, &comic.workset_id);
            }

            // Resolve comic team through the preloaded workset.
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

            // Populate chapter creator relation.
            AssignmentInclOpt::ChapterCreator => {
                //
                let Some(chapter) = &mut assignment.chapter else {
                    continue;
                };

                chapter.creator = find_user(state, &chapter.creator_id);
            }

            // Populate comic creator relation.
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

// Find user by id for assignment creator/user relations.
fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    state.users.iter().find(|info| info.id == user_id).cloned()
}

// Find chapter by id and clear its nested fields before explicit include expansion.
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

// Find comic by id and keep include-safe null defaults.
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

// Find workset used by comic-workset include resolution.
fn find_workset(state: &MockState, id: &str) -> Option<WorksetInfo> {
    state.worksets.iter().find(|info| info.id == id).cloned()
}

// Find team from a loaded workset.
fn find_team(state: &MockState, workset: &WorksetInfo) -> Option<TeamInfo> {
    state
        .teams
        .iter()
        .find(|info| info.id == workset.team_id)
        .cloned()
}
