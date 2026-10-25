//! Mock implementation of `PageSourceRepo`.

// Internal organization of the `orchestra` module.
mod orchestra;

use poprako_orchestra::{Run, Step};

use crate::model::read::proj::page_source::{
    PageSourceInfo, PageSourceRawIdentInfo, PageSourceUnitDiffStats,
    PageSourceUnitFlaggedStats, PageSourceUnitScope,
};
use crate::model::read::proj::unit::{UnitCountMetrics, has_unit_text};
use crate::model::write::page_source::PageSourceManifestEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page_source::{
    ListPageSourceRawIdentInfos, UpdatePageSourceRawIdents,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now, unrecoverable,
};
use crate::result::{BaseError, BaseRest};
use crate::value::page_source::MAX_CHAPTER_PAGE_SOURCE_COUNT;

// Internal implementation of `list_infos`.
// Look up page info by primary key; returns a business error on miss.
fn list_infos(state: &MockState, chapter_id: &str) -> Vec<PageSourceInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let mut page_source_infos = state
        .pages
        .iter()
        .filter(|page_source_info| page_source_info.chapter_id == chapter_id)
        .cloned()
        .collect::<Vec<_>>();

    page_source_infos.sort_by(|left, right| {
        (left.index, left.id.as_str()).cmp(&(right.index, right.id.as_str()))
    });

    page_source_infos
}

// List a complete valid Chapter manifest, retaining one sentinel PageSource for corruption detection.
fn list_bounded_infos(
    state: &MockState,
    chapter_id: &str,
) -> BaseRest<Vec<PageSourceInfo>> {
    //
    let page_source_infos = list_infos(state, chapter_id)
        .into_iter()
        .take(MAX_CHAPTER_PAGE_SOURCE_COUNT + 1)
        .collect::<Vec<_>>();

    if page_source_infos.len() > MAX_CHAPTER_PAGE_SOURCE_COUNT {
        //
        tracing::error!(
            chapter_id = %chapter_id,
            page_source_count_lower_bound = page_source_infos.len(),
            max_page_source_count = MAX_CHAPTER_PAGE_SOURCE_COUNT,
            "persisted Chapter PageSource count exceeds the business maximum",
        );

        return Err(unrecoverable(
            "persisted Chapter PageSource count exceeds the business maximum",
        ));
    }

    accept!(page_source_infos)
}

// Read detailed info by page primary key.
fn get_page_source_by_id(
    state: &MockState,
    id: &str,
) -> BaseRest<PageSourceInfo> {
    //
    state
        .pages
        .iter()
        .find(|page_source_info| {
            //
            page_source_info.id == id
                && !state
                    .deleted_chapter_ids
                    .contains(&page_source_info.chapter_id)
        })
        .cloned()
        .ok_or_else(|| expected("error-page-not-found"))
}

// Count all visible Units, then retain PageSources with revision differences.
fn list_unit_diff_stats(
    state: &MockState,
    chapter_id: &str,
) -> BaseRest<Vec<PageSourceUnitDiffStats>> {
    //
    let page_source_unit_diff_stats = list_bounded_infos(state, chapter_id)?
        .into_iter()
        .map(|page_source_info| {
            //
            let mut unit_diff_stats = PageSourceUnitDiffStats {
                page_source_id: page_source_info.id,
                index: page_source_info.index,
                translated_unit_count: 0,
                editted_unit_count: 0,
                proofreader_append_unit_count: 0,
            };

            for unit_info in state.units.iter().filter(|unit_info| {
                //
                unit_info.page_source_id == unit_diff_stats.page_source_id
                    && unit_info.hidden_at.is_none()
            }) {
                //
                let has_translation =
                    has_unit_text(unit_info.translated_text.as_deref());

                let has_revision =
                    has_unit_text(unit_info.proofread_text.as_deref());

                unit_diff_stats.translated_unit_count += usize::from(
                    has_translation
                        || has_unit_text(unit_info.typography_note.as_deref()),
                );

                unit_diff_stats.editted_unit_count += usize::from(
                    has_translation
                        && has_revision
                        && unit_info.translated_text
                            != unit_info.proofread_text,
                );

                unit_diff_stats.proofreader_append_unit_count +=
                    usize::from(!has_translation && has_revision);
            }

            unit_diff_stats
        })
        .filter(|unit_diff_stats| {
            //
            unit_diff_stats.editted_unit_count > 0
                || unit_diff_stats.proofreader_append_unit_count > 0
        })
        .collect();

    accept!(page_source_unit_diff_stats)
}

// Read the minimal PageSource scope used by Unit operations.
fn get_page_source_unit_scope(
    state: &MockState,
    id: &str,
) -> BaseRest<PageSourceUnitScope> {
    //
    let page_source_info = get_page_source_by_id(state, id)?;

    accept!(PageSourceUnitScope {
        id: page_source_info.id,
        chapter_id: page_source_info.chapter_id,
        count_metrics: UnitCountMetrics {
            total: page_source_info.total_unit_count,
            translated: page_source_info.translated_unit_count,
            proofread: page_source_info.proofread_unit_count,
        },
    })
}

// Internal implementation of `list_first_page_sources`.
fn list_first_page_sources(
    state: &MockState,
    chapter_ids: &[&str],
) -> Vec<PageSourceInfo> {
    //
    chapter_ids
        .iter()
        .filter_map(|chapter_id| {
            list_infos(state, chapter_id).into_iter().next()
        })
        .collect()
}

// Builds a new page projection for a final manifest entry.
fn page_source_from_manifest_entry(
    entry: &PageSourceManifestEntry,
) -> PageSourceInfo {
    //
    let time = now();

    PageSourceInfo {
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

impl Run<ListPageSourceRawIdentInfos<'_>> for Mock {
    // Shared application error type.
    type Error = BaseError;

    // Returns associations only for the requested pages.
    async fn run(
        &self,
        oper: &ListPageSourceRawIdentInfos<'_>,
    ) -> BaseRest<Vec<PageSourceRawIdentInfo>> {
        //
        let state = self.state.lock().unwrap();

        accept!(
            state
                .page_source_raw_idents
                .values()
                .filter(|info| oper
                    .page_source_ids
                    .contains(&info.page_source_id.as_str()))
                .cloned()
                .collect(),
        )
    }
}

impl Step<UpdatePageSourceRawIdents<'_>, MockContext> for Mock {
    // Matches the allocation transaction isolation requirement.
    type Level = ReptRead;

    // Shared application error type.
    type Error = BaseError;

    // Mutates the transaction-local associations with foreign-key parity.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdatePageSourceRawIdents<'_>,
    ) -> BaseRest<()> {
        //
        for &(page_source_id, raw_ident) in &oper.repl.idents {
            //
            match raw_ident {
                //
                None => {
                    context.state.page_source_raw_idents.remove(page_source_id);
                }

                Some(value) => {
                    //
                    if !context
                        .state
                        .pages
                        .iter()
                        .any(|page| page.id == page_source_id)
                    {
                        return Err(unrecoverable(
                            "raw filename references a missing page",
                        ));
                    }

                    let timestamp = now();

                    let info = context
                        .state
                        .page_source_raw_idents
                        .entry(page_source_id.to_owned())
                        .or_insert_with(|| PageSourceRawIdentInfo {
                            page_source_id: page_source_id.to_owned(),
                            raw_ident: value.to_owned(),
                            created_at: timestamp,
                            updated_at: timestamp,
                        });

                    info.raw_ident = value.to_owned();

                    info.updated_at = timestamp;
                }
            }
        }

        accept!(())
    }
}

// Count visible flagged Units without changing the original PageSource positions.
fn list_unit_flagged_stats(
    state: &MockState,
    chapter_id: &str,
) -> BaseRest<Vec<PageSourceUnitFlaggedStats>> {
    //
    let page_source_unit_flagged_stats = list_bounded_infos(state, chapter_id)?
        .into_iter()
        .map(|page_source_info| {
            //
            let flagged_unit_count = state
                .units
                .iter()
                .filter(|unit_info| {
                    //
                    unit_info.page_source_id == page_source_info.id
                        && unit_info.hidden_at.is_none()
                        && unit_info.is_flagged
                })
                .count();

            PageSourceUnitFlaggedStats {
                page_source_id: page_source_info.id,
                index: page_source_info.index,
                flagged_unit_count,
            }
        })
        .filter(|stats| stats.flagged_unit_count > 0)
        .collect();

    accept!(page_source_unit_flagged_stats)
}
