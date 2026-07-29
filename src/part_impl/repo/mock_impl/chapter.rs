//! In-memory chapter repository helpers.

use std::cmp::Reverse;

use crate::model::chapter::{ChapterEntry, ChapterInfo};
use crate::model::comic::ComicInfo;
use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::model::workset::WorksetInfo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{BaseResult, accept};
use crate::value::chapter::{ChapterInclOpt, StageMask};
use crate::value::incl::expand_incl_opts;

mod orchestra;

impl ChapterRepo<MockContext> for Mock {}

/// Looks up a chapter by id from the mock state, applying include options to resolve relations.
pub(super) fn get_chapter_by_id(
    state: &MockState,
    id: &str,
    incl_opt: &[ChapterInclOpt],
) -> BaseResult<ChapterInfo> {
    //
    let mut chapter_info = state
        .chapters
        .iter()
        .find(|chapter_info| chapter_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-chapter-not-found"))?;

    apply_chapter_incls(state, &mut chapter_info, incl_opt);

    accept(chapter_info)
}

/// Returns all chapters for a comic from the mock state, sorted by index descending.
pub(super) fn list_all_chapters(
    state: &MockState,
    comic_id: &str,
) -> Vec<ChapterInfo> {
    //
    let mut chapter_infos = state
        .chapters
        .iter()
        .filter(|chapter_info| chapter_info.comic_id == comic_id)
        .cloned()
        .collect::<Vec<_>>();

    chapter_infos.sort_by_key(|right| Reverse(right.index));

    chapter_infos
}

/// Inserts a new chapter into the mock state, returning the created [`ChapterInfo`].
pub(super) fn create_chapter(
    state: &mut MockState,
    chapter_entry: &ChapterEntry,
) -> BaseResult<ChapterInfo> {
    //
    if state
        .chapters
        .iter()
        .any(|chapter_info| chapter_info.id == chapter_entry.id)
    {
        return Err(expected("error-already-exists"));
    }

    let time = now();

    let chapter_info = ChapterInfo {
        id: chapter_entry.id.clone(),
        comic_id: chapter_entry.comic_id.clone(),
        comic: None,
        is_pinned: chapter_entry.is_pinned,
        index: chapter_entry.index,
        subtitle: chapter_entry.subtitle.clone(),
        page_count: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: StageMask::try_from(0u32).ok().unwrap(),
        creator_id: chapter_entry.creator_id.clone(),
        creator: None,
        created_at: time,
        updated_at: time,
    };

    state.chapters.push(chapter_info.clone());

    accept(chapter_info)
}

fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

fn find_comic(state: &MockState, comic_id: &str) -> Option<ComicInfo> {
    //
    let mut comic_info = state
        .comics
        .iter()
        .find(|comic_info| comic_info.id == comic_id)
        .cloned()?;

    comic_info.workset = None;

    comic_info.team = None;

    comic_info.creator = None;

    Some(comic_info)
}

fn find_workset(state: &MockState, workset_id: &str) -> Option<WorksetInfo> {
    state
        .worksets
        .iter()
        .find(|workset_info| workset_info.id == workset_id)
        .cloned()
}

fn find_team_for_workset(
    state: &MockState,
    workset_info: &WorksetInfo,
) -> Option<TeamInfo> {
    state
        .teams
        .iter()
        .find(|team_info| team_info.id == workset_info.team_id)
        .cloned()
}

fn apply_creator_incl(
    state: &MockState,
    chapter_info: &mut ChapterInfo,
    include_creator: bool,
) {
    //
    chapter_info.creator = None;

    if include_creator {
        chapter_info.creator = find_user(state, &chapter_info.creator_id);
    }
}

fn apply_comic_incl(
    state: &MockState,
    chapter_info: &mut ChapterInfo,
    include_comic: bool,
) {
    //
    chapter_info.comic = None;

    if include_comic {
        chapter_info.comic = find_comic(state, &chapter_info.comic_id);
    }
}

fn apply_comic_workset_incl(
    state: &MockState,
    chapter_info: &mut ChapterInfo,
    include_workset: bool,
) {
    //
    if !include_workset {
        return;
    }

    let Some(comic_info) = &mut chapter_info.comic else {
        return;
    };

    comic_info.workset = find_workset(state, &comic_info.workset_id);
}

fn apply_comic_workset_team_incl(
    state: &MockState,
    chapter_info: &mut ChapterInfo,
    include_team: bool,
) {
    //
    if !include_team {
        return;
    }

    let Some(comic_info) = &mut chapter_info.comic else {
        return;
    };

    let Some(workset_info) = &comic_info.workset else {
        return;
    };

    comic_info.team = find_team_for_workset(state, workset_info);
}

fn apply_comic_creator_incl(
    state: &MockState,
    chapter_info: &mut ChapterInfo,
    include_creator: bool,
) {
    //
    if !include_creator {
        return;
    }

    let Some(comic_info) = &mut chapter_info.comic else {
        return;
    };

    comic_info.creator = find_user(state, &comic_info.creator_id);
}

/// Applies the requested include options to a [`ChapterInfo`], resolving
/// comic, workset, team, and creator relations from the mock state.
pub(super) fn apply_chapter_incls(
    state: &MockState,
    chapter_info: &mut ChapterInfo,
    incl_opt: &[ChapterInclOpt],
) {
    //
    chapter_info.comic = None;

    chapter_info.creator = None;

    for incl_opt in expand_incl_opts(incl_opt) {
        match incl_opt {
            //
            ChapterInclOpt::Comic => {
                apply_comic_incl(state, chapter_info, true)
            }

            ChapterInclOpt::ComicWorkset => {
                apply_comic_workset_incl(state, chapter_info, true)
            }

            ChapterInclOpt::ComicWorksetTeam => {
                apply_comic_workset_team_incl(state, chapter_info, true)
            }

            ChapterInclOpt::ComicCreator => {
                apply_comic_creator_incl(state, chapter_info, true)
            }

            ChapterInclOpt::Creator => {
                apply_creator_incl(state, chapter_info, true)
            }
        }
    }
}
