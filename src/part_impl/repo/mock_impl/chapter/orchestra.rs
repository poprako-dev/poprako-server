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

fn list_chapter_infos(
    state: &MockState,
    spec: &ChapterListSpec,
) -> Vec<ChapterInfo> {
    //
    let mut chapter_infos = list_infos(state, &spec.comic_id);

    for chapter_info in &mut chapter_infos {
        apply_chapter_incls(state, chapter_info, &spec.incl);
    }

    let offset = spec.offset as usize;

    let limit = spec.limit.get() as usize;

    if offset >= chapter_infos.len() {
        Vec::new()
    } else {
        //
        let end = std::cmp::min(offset + limit, chapter_infos.len());

        (*chapter_infos.get(offset..end).unwrap()).to_vec()
    }
}

fn find_pinned_chapter_info(
    state: &MockState,
    comic_id: &str,
    incls: &[ChapterInclOpt],
) -> Option<ChapterInfo> {
    //
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

fn list_pinned_chapter_infos(
    state: &MockState,
    comic_ids: &[&str],
) -> Vec<ChapterInfo> {
    //
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
