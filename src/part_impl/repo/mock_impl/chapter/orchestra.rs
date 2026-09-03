// Run orchestration for chapter opers.
mod run;
// Step orchestration for chapter opers.
mod step;

use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::spec::chapter::ChapterListSpec;
use crate::part_impl::repo::mock_impl::MockState;
use crate::part_impl::repo::mock_impl::chapter::{
    apply_chapter_incls, list_infos,
};
use crate::value::chapter::ChapterInclOpt;

// Internal implementation of `list_chapter_infos`.
fn list_chapter_infos(
    state: &MockState,
    spec: &ChapterListSpec,
) -> Vec<ChapterInfo> {
    //
    let mut chapter_infos = list_infos(state, &spec.comic_id);

    for chapter_info in &mut chapter_infos {
        apply_chapter_incls(state, chapter_info, &spec.incl_opt);
    }

    let offset = spec.offset as usize;

    let limit = spec.limit as usize;

    if offset >= chapter_infos.len() {
        Vec::new()
    } else {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let end = std::cmp::min(offset + limit, chapter_infos.len());

        chapter_infos[offset..end].to_vec()
    }
}

// Internal implementation of `find_pinned_chapter_info`.
fn find_pinned_chapter_info(
    state: &MockState,
    comic_id: &str,
    incls: &[ChapterInclOpt],
) -> Option<ChapterInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let mut chapter_info = state
        .chapters
        .iter()
        .find(|chapter_info| {
            //
            chapter_info.comic_id == comic_id
                && chapter_info.is_pinned
                && !state.deleted_chapter_ids.contains(&chapter_info.id)
        })
        .cloned();

    if let Some(chapter_info) = &mut chapter_info {
        apply_chapter_incls(state, chapter_info, incls);
    }

    chapter_info
}

// Internal implementation of `list_pinned_chapter_infos`.
fn list_pinned_chapter_infos(
    state: &MockState,
    comic_ids: &[&str],
) -> Vec<ChapterInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    comic_ids
        .iter()
        .filter_map(|comic_id| {
            //
            state
                .chapters
                .iter()
                .find(|chapter_info| {
                    //
                    chapter_info.comic_id == *comic_id
                        && chapter_info.is_pinned
                        && !state.deleted_chapter_ids.contains(&chapter_info.id)
                })
                .cloned()
        })
        .collect()
}
