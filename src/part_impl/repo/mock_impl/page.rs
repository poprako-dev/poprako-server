//! Mock implementation of `PageRepo`.

// Internal organization of the `orchestra` module.
mod orchestra;

use crate::model::read::proj::page::PageInfo;
use crate::model::write::page::{PageEntry, PageManifestEntry};
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

// List Chapter Page IDs containing at least one visible text diff.
fn list_editted_diff_page_ids(
    state: &MockState,
    chapter_id: &str,
) -> Vec<String> {
    //
    list_infos(state, chapter_id)
        .into_iter()
        .filter(|page_info| {
            //
            // Stop checking this Page after its first matching Unit.
            state.units.iter().any(|unit_info| {
                //
                unit_info.page_id == page_info.id
                    && unit_info.hidden_at.is_none()
                    && unit_info.proofread_text.as_deref().is_some_and(
                        |proofread_text| {
                            //
                            !proofread_text.trim().is_empty()
                                && Some(proofread_text)
                                    != unit_info.translated_text.as_deref()
                        },
                    )
            })
        })
        .map(|page_info| page_info.id)
        .collect()
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
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    }
}

// Builds a new page projection for a final manifest entry.
fn page_from_manifest_entry(entry: &PageManifestEntry) -> PageInfo {
    //
    let time = now();

    PageInfo {
        id: entry.id.clone(),
        chapter_id: entry.chapter_id.clone(),
        index: entry.index,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    }
}
