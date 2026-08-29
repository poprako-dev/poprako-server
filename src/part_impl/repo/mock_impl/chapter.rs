//! In-memory chapter repository helpers.

// Internal organization of the `orchestra` module.
mod orchestra;

use std::cmp::Reverse;

use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::user::UserInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::write::chapter::ChapterEntry;
use crate::part_impl::repo::mock_impl::{MockState, expected, now};
use crate::result::{BaseRest, accept};
use crate::value::chapter::ChapterInclOpt;
use crate::value::chapter::mask::StageMask;
use crate::value::incl::expand_incl_opts;

/// Looks up a chapter by id from the mock state, applying include options to
/// resolve relations.
pub fn get_chapter_by_id(
    state: &MockState,
    id: &str,
    incl_opt: &[ChapterInclOpt],
) -> BaseRest<ChapterInfo> {
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

/// Returns chapters for a comic from the mock state, sorted by index descending.
pub fn list_infos(state: &MockState, comic_id: &str) -> Vec<ChapterInfo> {
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
pub fn create_chapter(
    state: &mut MockState,
    chapter_entry: &ChapterEntry,
) -> BaseRest<ChapterInfo> {
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

/// Enriches chapter include fields for the requested relation set.
pub fn apply_chapter_incls(
    state: &MockState,
    chapter_info: &mut ChapterInfo,
    incl_opt: &[ChapterInclOpt],
) {
    //
    chapter_info.comic = None;

    chapter_info.creator = None;

    for incl_opt in expand_incl_opts(incl_opt) {
        //
        match incl_opt {
            //
            // Attach base comic fields.
            ChapterInclOpt::Comic => {
                apply_comic_incl(state, chapter_info, true);
            }

            // Attach comic workspace relation.
            ChapterInclOpt::ComicWorkset => {
                apply_comic_workset_incl(state, chapter_info, true);
            }

            // Attach comic workspace team relation.
            ChapterInclOpt::ComicWorksetTeam => {
                apply_comic_workset_team_incl(state, chapter_info, true);
            }

            // Attach comic creator relation.
            ChapterInclOpt::ComicCreator => {
                apply_comic_creator_incl(state, chapter_info, true);
            }

            // Attach chapter creator relation.
            ChapterInclOpt::Creator => {
                apply_creator_incl(state, chapter_info, true);
            }
        }
    }
}

// Apply comic include and clear previous value first.
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

// Apply comic->workset include.
fn apply_comic_workset_incl(
    state: &MockState,
    chapter_info: &mut ChapterInfo,
    include_workset: bool,
) {
    //
    if !include_workset {
        return;
    }

    let Some(comic) = chapter_info.comic.as_mut() else {
        return;
    };

    comic.workset = find_workset(state, &comic.workset_id);
}

// Apply comic->team include through comic workset.
fn apply_comic_workset_team_incl(
    state: &MockState,
    chapter_info: &mut ChapterInfo,
    include_team: bool,
) {
    //
    if !include_team {
        return;
    }

    let Some(comic) = chapter_info.comic.as_mut() else {
        return;
    };

    let Some(workset) = &comic.workset else {
        return;
    };

    comic.team = find_team(state, workset);
}

// Apply comic creator relation when requested.
fn apply_comic_creator_incl(
    state: &MockState,
    chapter_info: &mut ChapterInfo,
    include_creator: bool,
) {
    //
    if !include_creator {
        return;
    }

    let Some(comic) = chapter_info.comic.as_mut() else {
        return;
    };

    comic.creator = find_user(state, &comic.creator_id);
}

// Apply user creator include for a chapter.
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

// Resolve comic details for include branches.
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

// Resolve a workset for comic include expansion.
fn find_workset(state: &MockState, id: &str) -> Option<WorksetInfo> {
    state.worksets.iter().find(|info| info.id == id).cloned()
}

// Resolve team by workset id.
fn find_team(state: &MockState, workset: &WorksetInfo) -> Option<TeamInfo> {
    //
    state
        .teams
        .iter()
        .find(|info| info.id == workset.team_id)
        .cloned()
}

// Resolve user and chapter creator relations first.
fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    //
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}
