//! Mock implementation of `PageRepo`.

use std::collections::HashMap;

use crate::model::page::{PageEntry, PageInfo};
use crate::part::repo::page::PageRepo;
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::RegularResult;

impl PageRepo<MockContext> for Mock {}

mod orchestra;

fn get_page_by_id(state: &MockState, id: &str) -> RegularResult<PageInfo> {
    state
        .pages
        .iter()
        .find(|page_info| page_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-page-not-found"))
}

fn list_all_pages(state: &MockState, chapter_id: &str) -> Vec<PageInfo> {
    //
    let mut page_infos = state
        .pages
        .iter()
        .filter(|page_info| page_info.chapter_id == chapter_id)
        .cloned()
        .collect::<Vec<_>>();

    page_infos.sort_by_key(|left| left.index);

    page_infos
}

fn list_pages(
    state: &MockState,
    chapter_id: &str,
    offset: u32,
    limit: u32,
) -> Vec<PageInfo> {
    //
    let page_infos = list_all_pages(state, chapter_id);

    let offset = offset as usize;

    let limit = limit as usize;

    if offset >= page_infos.len() {
        return Vec::new();
    }

    let end = std::cmp::min(offset + limit, page_infos.len());

    page_infos[offset..end].to_vec()
}

fn list_first_pages(
    state: &MockState,
    chapter_ids: &[String],
) -> HashMap<String, PageInfo> {
    //
    chapter_ids
        .iter()
        .filter_map(|chapter_id| {
            list_all_pages(state, chapter_id)
                .into_iter()
                .next()
                .map(|page_info| (chapter_id.clone(), page_info))
        })
        .collect()
}

fn page_from_entry(entry: &PageEntry) -> PageInfo {
    //
    let time = now();

    PageInfo {
        id: entry.id.clone(),
        chapter_id: entry.chapter_id.clone(),
        index: entry.index,
        image_key: entry.image_key.clone(),
        image_uploaded: false,
        image_version: entry.image_version,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    }
}
