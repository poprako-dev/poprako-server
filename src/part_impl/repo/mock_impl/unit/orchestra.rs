use std::collections::HashSet;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::unit::{UnitCountMetrics, UnitInfo, UnitOrder};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfosByIds, ListUnitInfosByPageSourceIds,
    ListUnitInfosInChapterOrder, ListUnitOrders, SearchChapterUnitIds,
};
use crate::part_impl::repo::mock_impl::unit::{
    apply_edits, list_infos, list_infos_by_ids, list_infos_by_page_source_ids,
    list_orders,
};
use crate::part_impl::repo::mock_impl::{Mock, MockContext, MockState};
use crate::result::{BaseError, BaseRest};
use crate::value::unit::UnitTextPart;

// Reports whether the selected Unit text contains the literal phrase.
fn text_part_contains(
    unit_info: &UnitInfo,
    part: UnitTextPart,
    phrase: &str,
) -> bool {
    //
    match part {
        //
        UnitTextPart::TranslatedText => unit_info
            .translated_text
            .as_deref()
            .is_some_and(|text| text.contains(phrase)),

        //
        UnitTextPart::ProofreadText => unit_info
            .proofread_text
            .as_deref()
            .is_some_and(|text| text.contains(phrase)),
    }
}

// Searches visible Unit IDs in stable Chapter order up to a fetch bound.
fn search_chapter_ids(
    state: &MockState,
    chapter_id: &str,
    part: UnitTextPart,
    phrase: &str,
    fetch_count: usize,
) -> BaseRest<Vec<String>> {
    //
    let mut pages = state
        .pages
        .iter()
        .filter(|page_source_info| page_source_info.chapter_id == chapter_id)
        .collect::<Vec<_>>();

    pages.sort_by_key(|page_source_info| page_source_info.index);

    let mut ids = Vec::new();

    for page_source_info in pages {
        //
        for unit_info in list_infos(state, &page_source_info.id)? {
            //
            if unit_info.hidden_at.is_some()
                || !text_part_contains(unit_info, part, phrase)
            {
                continue;
            }

            ids.push(unit_info.id.clone());

            if ids.len() >= fetch_count {
                return accept!(ids);
            }
        }
    }

    accept!(ids)
}

// Loads selected Unit infos in stable Chapter PageSource and linked-list order.
fn list_infos_in_chapter_order(
    state: &MockState,
    ids: &[&str],
) -> BaseRest<Vec<UnitInfo>> {
    //
    let page_source_ids = state
        .units
        .iter()
        .filter(|unit_info| ids.contains(&unit_info.id.as_str()))
        .map(|unit_info| unit_info.page_source_id.as_str())
        .collect::<HashSet<_>>();

    let mut pages = state
        .pages
        .iter()
        .filter(|page_source_info| {
            page_source_ids.contains(page_source_info.id.as_str())
        })
        .collect::<Vec<_>>();

    pages.sort_by_key(|page_source_info| page_source_info.index);

    let mut selected_infos = Vec::new();

    for page_source_info in pages {
        //
        selected_infos.extend(
            list_infos(state, &page_source_info.id)?
                .into_iter()
                .filter(|unit_info| {
                    //
                    unit_info.hidden_at.is_none()
                        && ids.contains(&unit_info.id.as_str())
                })
                .cloned(),
        );
    }

    accept!(selected_infos)
}

impl Run<ListUnitInfosByPageSourceIds<'_>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListUnitInfosByPageSourceIds<'_>,
    ) -> BaseRest<Vec<UnitInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        list_infos_by_page_source_ids(&state, oper.page_source_ids)
    }
}

impl Step<ListUnitInfosByIds<'_>, MockContext> for Mock {
    // Internal type alias for `Level`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Lists every requested Unit that exists in the transaction snapshot.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListUnitInfosByIds<'_>,
    ) -> BaseRest<Vec<UnitInfo>> {
        accept!(list_infos_by_ids(&context.state, oper.ids))
    }
}

impl Step<SearchChapterUnitIds<'_>, MockContext> for Mock {
    // Minimum transaction level for coherent search reads.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Searches visible Unit IDs within the Chapter scope.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &SearchChapterUnitIds<'_>,
    ) -> BaseRest<Vec<String>> {
        //
        search_chapter_ids(
            &context.state,
            oper.chapter_id,
            oper.part,
            oper.phrase,
            oper.fetch_count,
        )
    }
}

impl Step<ListUnitInfosInChapterOrder<'_>, MockContext> for Mock {
    // Minimum transaction level for coherent ordered search reads.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Loads selected Unit infos in Chapter presentation order.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListUnitInfosInChapterOrder<'_>,
    ) -> BaseRest<Vec<UnitInfo>> {
        list_infos_in_chapter_order(&context.state, oper.ids)
    }
}

impl Step<ListUnitOrders<'_>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListUnitOrders<'_>,
    ) -> BaseRest<Vec<UnitOrder>> {
        list_orders(&context.state, oper.page_source_id)
    }
}

impl Step<ApplyUnitEdits<'_>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ApplyUnitEdits<'_>,
    ) -> BaseRest<UnitCountMetrics> {
        //
        apply_edits(
            &mut context.state,
            oper.page_source_id,
            oper.orders,
            oper.edits,
        )
    }
}
