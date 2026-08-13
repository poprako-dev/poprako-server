// Run orchestration for comic opers.
mod run;
// Step orchestration for comic opers.
mod step;

use crate::complex::comic::ComicComplex;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::user::UserInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::read::spec::comic::ComicListSpec;
use crate::part_impl::repo::mock_impl::{MockState, expected, now};
use crate::result::{BaseRest, accept};
use crate::value::chapter::StageMask;
use crate::value::comic::{ComicInclOpt, ComicStatus};
use crate::value::incl::expand_incl_opts;
use crate::value::index::user_index_to_stored_index;

// Find and clone a workset from mock storage by id.
fn find_workset(state: &MockState, workset_id: &str) -> Option<WorksetInfo> {
    //
    state
        .worksets
        .iter()
        .find(|workset_info| workset_info.id == workset_id)
        .cloned()
}

// Resolve the team owner of a workset for relation enrichment.
fn find_team_for_workset(
    state: &MockState,
    workset: &WorksetInfo,
) -> Option<TeamInfo> {
    //
    state
        .teams
        .iter()
        .find(|team_info| team_info.id == workset.team_id)
        .cloned()
}

// Resolve a user from mock state for creator relation enrichment.
fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    //
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

// Populate workset field when workset include is requested.
fn apply_workset_incl(
    state: &MockState,
    comic_info: &mut ComicInfo,
    include_workset: bool,
) {
    //
    comic_info.workset = None;

    if include_workset {
        comic_info.workset = find_workset(state, &comic_info.workset_id);
    }
}

// Populate team field when team include is requested.
fn apply_team_incl(
    state: &MockState,
    comic_info: &mut ComicInfo,
    include_team: bool,
) {
    //
    comic_info.team = None;

    if !include_team {
        return;
    }

    let Some(workset_info) = &comic_info.workset else {
        return;
    };

    comic_info.team = find_team_for_workset(state, workset_info);
}

// Populate creator/workset/team fields according to include options.
fn apply_creator_incl(
    state: &MockState,
    comic_info: &mut ComicInfo,
    include_creator: bool,
) {
    //
    comic_info.creator = None;

    if include_creator {
        comic_info.creator = find_user(state, &comic_info.creator_id);
    }
}

// Apply relation includes to a comic summary in a stable order.
fn apply_comic_incls(
    state: &MockState,
    comic_info: &mut ComicInfo,
    incl_opt: &[ComicInclOpt],
) {
    //
    comic_info.workset = None;

    comic_info.team = None;

    comic_info.creator = None;

    for incl_opt in expand_incl_opts(incl_opt) {
        //
        match incl_opt {
            //
            ComicInclOpt::Workset => {
                apply_workset_incl(state, comic_info, true)
            }

            ComicInclOpt::WorksetTeam => {
                apply_team_incl(state, comic_info, true)
            }

            ComicInclOpt::Creator => {
                apply_creator_incl(state, comic_info, true)
            }
        }
    }
}

// Check title/index fuzzy condition for list filtering.
fn comic_matches_fuzzy(comic_info: &ComicInfo, fuzzy_title: &str) -> bool {
    //
    let composed_title = ComicComplex::compose_title(
        comic_info.index,
        &comic_info.author,
        &comic_info.title,
    )
    .to_lowercase();

    let fuzzy_title = fuzzy_title.to_lowercase();

    if composed_title.contains(fuzzy_title.as_str()) {
        return true;
    }

    match fuzzy_title.trim().parse() {
        //
        Ok(index) => user_index_to_stored_index(index)
            .map(|index| comic_info.index == index)
            .unwrap_or(false),

        Err(_) => false,
    }
}

// Check whether a comic matches list scope constraints.
fn comic_matches_stages(
    state: &MockState,
    comic_info: &ComicInfo,
    stages: Option<StageMask>,
) -> bool {
    //
    match stages {
        //
        Some(stage_mask) => state
            .chapters
            .iter()
            .find(|chapter_info| {
                chapter_info.comic_id == comic_info.id && chapter_info.is_pinned
            })
            .map(|chapter_info| chapter_info.stages.matches_filter(stage_mask))
            .unwrap_or(false),

        None => true,
    }
}

// Check whether a comic matches its requested lifecycle status.
fn comic_matches_status(
    comic_info: &ComicInfo,
    status: Option<ComicStatus>,
) -> bool {
    //
    match status {
        //
        Some(ComicStatus::Active) => comic_info.archived_at.is_none(),

        Some(ComicStatus::Archived) => comic_info.archived_at.is_some(),

        None => true,
    }
}

// Validate optimistic fields and toggle comic cover uploaded flag.
fn mark_comic_cover_uploaded(
    state: &mut MockState,
    id: &str,
    cover_version: u32,
    cover_key: Option<&str>,
    cover_uploaded: bool,
) -> BaseRest<()> {
    //
    let comic = state
        .comics
        .iter_mut()
        .find(|comic| comic.id == id)
        .ok_or_else(|| expected("error-comic-not-found"))?;

    if comic.cover_version != Some(cover_version)
        || cover_key.is_some_and(|cover_key| {
            comic.cover_key.as_deref() != Some(cover_key)
        })
    {
        return Err(expected("error-stale-cover-upload"));
    }

    comic.is_cover_uploaded = Some(cover_uploaded);

    comic.updated_at = now();

    accept(())
}

// Load one comic and hydrate include fields.
fn get_comic_info(
    state: &MockState,
    id: &str,
    incls: &[ComicInclOpt],
) -> BaseRest<ComicInfo> {
    //
    let mut comic_info = state
        .comics
        .iter()
        .find(|comic_info| comic_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-comic-not-found"))?;

    apply_comic_incls(state, &mut comic_info, incls);

    accept(comic_info)
}

// Build filtered, sorted and paginated comic lists.
fn list_comic_infos(state: &MockState, spec: &ComicListSpec) -> Vec<ComicInfo> {
    //
    let mut comic_infos = state
        .comics
        .iter()
        .filter(|comic_info| comic_info.workset_id == spec.workset_id)
        .filter(|comic_info| {
            //
            spec.fuzzy_title
                .as_ref()
                .map(|keyword| comic_matches_fuzzy(comic_info, keyword))
                .unwrap_or(true)
        })
        .filter(|comic_info| {
            comic_matches_stages(state, comic_info, spec.stages)
        })
        .filter(|comic_info| comic_matches_status(comic_info, spec.status))
        .cloned()
        .collect::<Vec<_>>();

    comic_infos.sort_by(|left, right| {
        //
        right
            .last_active_at
            .cmp(&left.last_active_at)
            .then_with(|| left.index.cmp(&right.index))
    });

    for comic_info in &mut comic_infos {
        apply_comic_incls(state, comic_info, &spec.incl_opt);
    }

    let offset = spec.offset as usize;

    let limit = spec.limit as usize;

    match offset >= comic_infos.len() {
        //
        true => Vec::new(),

        false => {
            //
            let end = std::cmp::min(offset + limit, comic_infos.len());

            comic_infos[offset..end].to_vec()
        }
    }
}
