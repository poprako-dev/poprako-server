//! Mock implementation of `PageRepo`.

// Internal organization of the `orchestra` module.
mod orchestra;

use crate::model::read::proj::page::{PageInfo, PageUnitScope};
use crate::model::read::proj::unit::UnitCountMetrics;
use crate::model::write::page::PageManifestEntry;
use crate::part_impl::repo::mock_impl::{
    MockState, expected, now, unrecoverable,
};
use crate::result::{BaseRest, accept};
use crate::value::page::MAX_CHAPTER_PAGE_COUNT;

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

    page_infos.sort_by(|left, right| {
        (left.index, left.id.as_str()).cmp(&(right.index, right.id.as_str()))
    });

    page_infos
}

// List a complete valid Chapter manifest, retaining one sentinel Page for corruption detection.
fn list_bounded_infos(
    state: &MockState,
    chapter_id: &str,
) -> BaseRest<Vec<PageInfo>> {
    //
    let page_infos = list_infos(state, chapter_id)
        .into_iter()
        .take(MAX_CHAPTER_PAGE_COUNT + 1)
        .collect::<Vec<_>>();

    if page_infos.len() > MAX_CHAPTER_PAGE_COUNT {
        //
        tracing::error!(
            chapter_id = %chapter_id,
            page_count_lower_bound = page_infos.len(),
            max_page_count = MAX_CHAPTER_PAGE_COUNT,
            "persisted Chapter Page count exceeds the business maximum",
        );

        return Err(unrecoverable(
            "persisted Chapter Page count exceeds the business maximum",
        ));
    }

    accept(page_infos)
}

// Read detailed info by page primary key.
fn get_page_by_id(state: &MockState, id: &str) -> BaseRest<PageInfo> {
    //
    state
        .pages
        .iter()
        .find(|page_info| {
            //
            page_info.id == id
                && !state.deleted_chapter_ids.contains(&page_info.chapter_id)
        })
        .cloned()
        .ok_or_else(|| expected("error-page-not-found"))
}

// List Chapter Page IDs containing at least one visible text diff.
fn list_editted_diff_page_ids(
    state: &MockState,
    chapter_id: &str,
) -> BaseRest<Vec<String>> {
    //
    let page_ids = list_bounded_infos(state, chapter_id)?
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
        .collect();

    accept(page_ids)
}

// Read the minimal Page scope used by Unit operations.
fn get_page_unit_scope(state: &MockState, id: &str) -> BaseRest<PageUnitScope> {
    //
    let page_info = get_page_by_id(state, id)?;

    accept(PageUnitScope {
        id: page_info.id,
        chapter_id: page_info.chapter_id,
        count_metrics: UnitCountMetrics {
            total: page_info.total_unit_count,
            translated: page_info.translated_unit_count,
            proofread: page_info.proofread_unit_count,
        },
    })
}

// Internal implementation of `list_first_pages`.
fn list_first_pages(state: &MockState, chapter_ids: &[&str]) -> Vec<PageInfo> {
    //
    chapter_ids
        .iter()
        .filter_map(|chapter_id| {
            list_infos(state, chapter_id).into_iter().next()
        })
        .collect()
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
