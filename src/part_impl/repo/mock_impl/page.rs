//! Mock implementation of `PageRepo`.

// Internal organization of the `orchestra` module.
mod orchestra;

use crate::model::read::proj::page::PageInfo;
use crate::model::write::page::PageEntry;
use crate::part_impl::repo::mock_impl::{MockState, expected, now};
use crate::result::BaseRest;

// Internal implementation of `list_infos`.
// Look up page info by primary key; returns a business error on miss.
fn list_infos(state: &MockState, chapter_id: &str) -> Vec<PageInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let mut page_infos = state
        .pages
        .iter()
        .filter(|page_info| page_info.chapter_id == chapter_id)
        .cloned()
        .collect::<Vec<_>>();

    page_infos.sort_by_key(|left| left.index);

    page_infos
}

// Read detailed info by page primary key.
fn get_page_by_id(state: &MockState, id: &str) -> BaseRest<PageInfo> {
    //
    state
        .pages
        .iter()
        .find(|page_info| page_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-page-not-found"))
}

// Internal implementation of `list_first_pages`.
fn list_first_pages(
    state: &MockState,
    chapter_ids: &[String],
) -> Vec<PageInfo> {
    //
    chapter_ids
        .iter()
        .filter_map(|chapter_id| {
            list_infos(state, chapter_id).into_iter().next()
        })
        .collect()
}

// Internal implementation of `page_from_entry`.
fn page_from_entry(entry: &PageEntry) -> PageInfo {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let time = now();

    PageInfo {
        id: entry.id.clone(),
        chapter_id: entry.chapter_id.clone(),
        index: entry.index,
        image_key: entry.image_key.clone(),
        is_image_uploaded: Some(false),
        image_version: Some(entry.image_version),
        image_hash: Some(entry.image_hash.clone()),
        image_ext: Some(entry.image_ext),
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    }
}
