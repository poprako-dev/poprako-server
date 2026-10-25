use std::collections::HashSet;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::page_source::{
    PageSourceInfo, PageSourceUnitDiffStats, PageSourceUnitFlaggedStats,
    PageSourceUnitScope,
};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page_source::{
    ApplyPageSourceManifest, DeletePageSources, GetPageSourceInfo,
    GetPageSourceInfoExcluded, GetPageSourceUnitScope,
    GetPageSourceUnitScopeExcluded, ListFirstPageSourceInfos,
    ListPageSourceInfos, ListPageSourceInfosExcluded,
    ListPageSourceUnitDiffStats, ListPageSourceUnitFlaggedStats,
    SetPageSourceUnitCountMetrics, ShiftPageSourceIndexesTemporary,
};
use crate::part_impl::repo::mock_impl::page_source::{
    get_page_source_by_id, get_page_source_unit_scope, list_bounded_infos,
    list_first_page_sources, list_unit_diff_stats, list_unit_flagged_stats,
    page_source_from_manifest_entry,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, expected, now, unrecoverable,
};
use crate::result::{BaseError, BaseRest};

impl<'a> Run<GetPageSourceInfo<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &GetPageSourceInfo<'a>,
    ) -> BaseRest<PageSourceInfo> {
        //
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        get_page_source_by_id(&state, oper.id)
    }
}

impl Run<GetPageSourceUnitScope<'_>> for Mock {
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Loads the minimal PageSource scope needed by Unit operations.
    async fn run(
        &self,
        oper: &GetPageSourceUnitScope<'_>,
    ) -> BaseRest<PageSourceUnitScope> {
        //
        let state = self
            .state
            .lock()
            .map_err(|_| unrecoverable("mock state lock is poisoned"))?;

        get_page_source_unit_scope(&state, oper.id)
    }
}

impl<'a> Run<ListPageSourceInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListPageSourceInfos<'a>,
    ) -> BaseRest<Vec<PageSourceInfo>> {
        //
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        list_bounded_infos(&state, oper.chapter_id)
    }
}

impl<'a> Run<ListFirstPageSourceInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListFirstPageSourceInfos<'a>,
    ) -> BaseRest<Vec<PageSourceInfo>> {
        //
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept!(list_first_page_sources(&state, oper.chapter_ids))
    }
}

impl Run<ListPageSourceUnitDiffStats<'_>> for Mock {
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Lists matching PageSource text statistics in stable Chapter PageSource order.
    async fn run(
        &self,
        oper: &ListPageSourceUnitDiffStats<'_>,
    ) -> BaseRest<Vec<PageSourceUnitDiffStats>> {
        //
        let state = self.state.lock().unwrap();

        list_unit_diff_stats(&state, oper.chapter_id)
    }
}

impl Run<ListPageSourceUnitFlaggedStats<'_>> for Mock {
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Lists matching PageSource flagged Unit statistics in stable Chapter PageSource order.
    async fn run(
        &self,
        oper: &ListPageSourceUnitFlaggedStats<'_>,
    ) -> BaseRest<Vec<PageSourceUnitFlaggedStats>> {
        //
        let state = self.state.lock().unwrap();

        list_unit_flagged_stats(&state, oper.chapter_id)
    }
}

impl<'a> Step<GetPageSourceInfo<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetPageSourceInfo<'a>,
    ) -> BaseRest<PageSourceInfo> {
        get_page_source_by_id(&context.state, oper.id)
    }
}

impl Step<GetPageSourceUnitScope<'_>, MockContext> for Mock {
    // Declares the transaction isolation level required for this read.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Loads the minimal PageSource scope from transaction-local state.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetPageSourceUnitScope<'_>,
    ) -> BaseRest<PageSourceUnitScope> {
        get_page_source_unit_scope(&context.state, oper.id)
    }
}

impl<'a> Step<ListPageSourceInfos<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListPageSourceInfos<'a>,
    ) -> BaseRest<Vec<PageSourceInfo>> {
        list_bounded_infos(&context.state, oper.chapter_id)
    }
}

impl<'a> Step<ListPageSourceInfosExcluded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListPageSourceInfosExcluded<'a>,
    ) -> BaseRest<Vec<PageSourceInfo>> {
        list_bounded_infos(&context.state, oper.chapter_id)
    }
}

impl<'a> Step<GetPageSourceInfoExcluded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetPageSourceInfoExcluded<'a>,
    ) -> BaseRest<PageSourceInfo> {
        get_page_source_by_id(&context.state, oper.id)
    }
}

impl Step<GetPageSourceUnitScopeExcluded<'_>, MockContext> for Mock {
    // Declares the transaction isolation level required for this locked read.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Loads the minimal PageSource scope used by Unit edits.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetPageSourceUnitScopeExcluded<'_>,
    ) -> BaseRest<PageSourceUnitScope> {
        get_page_source_unit_scope(&context.state, oper.id)
    }
}

impl<'a> Step<SetPageSourceUnitCountMetrics<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &SetPageSourceUnitCountMetrics<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        let page_source_info = context
            .state
            .pages
            .iter_mut()
            .find(|info| info.id == oper.id)
            .ok_or_else(|| expected("error-page-not-found"))?;

        page_source_info.total_unit_count = oper.count_metrics.total;

        page_source_info.translated_unit_count = oper.count_metrics.translated;

        page_source_info.proofread_unit_count = oper.count_metrics.proofread;

        page_source_info.updated_at = now();

        accept!(())
    }
}

impl<'a> Step<ShiftPageSourceIndexesTemporary<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Move current page indexes out of the final nonnegative range.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ShiftPageSourceIndexesTemporary<'a>,
    ) -> BaseRest<()> {
        //
        for page_source_info in
            context.state.pages.iter_mut().filter(|page_source_info| {
                page_source_info.chapter_id == oper.chapter_id
            })
        {
            page_source_info.index = usize::MAX - page_source_info.index;

            page_source_info.updated_at = now();
        }

        accept!(())
    }
}

impl<'a> Step<ApplyPageSourceManifest<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Apply retained index updates and new page inserts as one mock operation.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ApplyPageSourceManifest<'a>,
    ) -> BaseRest<Vec<PageSourceInfo>> {
        //
        let mut seen_ids = HashSet::with_capacity(oper.entries.len());

        let mut page_source_infos = Vec::with_capacity(oper.entries.len());

        for entry in oper.entries {
            //
            let is_unique = seen_ids.insert(entry.id.as_str());

            if !is_unique {
                //
                return Err(unrecoverable(
                    "page manifest contains a duplicate page id",
                ));
            }

            let existing_page_source_info = context
                .state
                .pages
                .iter_mut()
                .find(|page_source_info| page_source_info.id == entry.id);

            let Some(page_source_info) = existing_page_source_info else {
                //
                let page_source_info = page_source_from_manifest_entry(entry);

                context.state.pages.push(page_source_info.clone());

                page_source_infos.push(page_source_info);

                continue;
            };

            if page_source_info.chapter_id != entry.chapter_id {
                //
                return Err(unrecoverable(
                    "page manifest cannot move a page between chapters",
                ));
            }

            page_source_info.index = entry.index;

            page_source_info.updated_at = now();

            page_source_infos.push(page_source_info.clone());
        }

        accept!(page_source_infos)
    }
}

impl<'a> Step<DeletePageSources<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeletePageSources<'a>,
    ) -> BaseRest<()> {
        //
        match oper {
            //
            // Internal implementation detail.
            DeletePageSources::Chapter { chapter_id } => {
                //
                // Internal implementation detail.
                let ids = context
                    .state
                    .pages
                    .iter()
                    .filter(|page_source_info| {
                        page_source_info.chapter_id == *chapter_id
                    })
                    .map(|page_source_info| page_source_info.id.clone())
                    .collect::<Vec<_>>();

                context.state.units.retain(|unit_info| {
                    !ids.contains(&unit_info.page_source_id)
                });

                context.state.pages.retain(|page_source_info| {
                    page_source_info.chapter_id != *chapter_id
                });

                context
                    .state
                    .page_source_raw_idents
                    .retain(|page_source_id, _| !ids.contains(page_source_id));

                accept!(())
            }

            DeletePageSources::Ids { ids } => {
                //
                // Internal implementation detail.
                context.state.units.retain(|unit_info| {
                    !ids.contains(&unit_info.page_source_id)
                });

                context.state.pages.retain(|page_source_info| {
                    !ids.contains(&page_source_info.id)
                });

                context
                    .state
                    .page_source_raw_idents
                    .retain(|page_source_id, _| !ids.contains(page_source_id));

                accept!(())
            }
        }
    }
}
