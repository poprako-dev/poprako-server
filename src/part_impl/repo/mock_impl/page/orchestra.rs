use std::collections::HashSet;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::page::PageInfo;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page::{
    ApplyPageManifest, CreatePages, DeletePages, GetPageInfo,
    GetPageInfoExcluded, ListFirstPageInfos, ListPageInfos,
    ListPageInfosExcluded, SetPageUnitCounters, ShiftPageIndexesTemporary,
};
use crate::part_impl::repo::mock_impl::page::{
    get_page_by_id, list_first_pages, list_infos, page_from_entry,
    page_from_manifest_entry,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, expected, now, unrecoverable,
};
use crate::result::{BaseError, BaseRest, accept};

impl<'a> Run<GetPageInfo<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &GetPageInfo<'a>) -> BaseRest<PageInfo> {
        //
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        get_page_by_id(&state, oper.id)
    }
}

impl<'a> Run<ListPageInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &ListPageInfos<'a>) -> BaseRest<Vec<PageInfo>> {
        //
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(list_infos(&state, oper.chapter_id))
    }
}

impl<'a> Run<ListFirstPageInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListFirstPageInfos<'a>,
    ) -> BaseRest<Vec<PageInfo>> {
        //
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(list_first_pages(&state, oper.chapter_ids))
    }
}

impl<'a> Step<GetPageInfo<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetPageInfo<'a>,
    ) -> BaseRest<PageInfo> {
        get_page_by_id(&context.state, oper.id)
    }
}

impl<'a> Step<ListPageInfos<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListPageInfos<'a>,
    ) -> BaseRest<Vec<PageInfo>> {
        accept(list_infos(&context.state, oper.chapter_id))
    }
}

impl<'a> Step<ListPageInfosExcluded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListPageInfosExcluded<'a>,
    ) -> BaseRest<Vec<PageInfo>> {
        accept(list_infos(&context.state, oper.chapter_id))
    }
}

impl<'a> Step<CreatePages<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreatePages<'a>,
    ) -> BaseRest<Vec<PageInfo>> {
        //
        // Internal implementation detail.
        if oper.entries.iter().any(|page_entry| {
            //
            context
                .state
                .pages
                .iter()
                .any(|page_info| page_info.id == page_entry.id)
        }) {
            return Err(expected("error-already-exists"));
        }

        let infos =
            oper.entries.iter().map(page_from_entry).collect::<Vec<_>>();

        context.state.pages.extend(infos.clone());

        accept(infos)
    }
}

impl<'a> Step<GetPageInfoExcluded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetPageInfoExcluded<'a>,
    ) -> BaseRest<PageInfo> {
        get_page_by_id(&context.state, oper.id)
    }
}

impl<'a> Step<SetPageUnitCounters<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &SetPageUnitCounters<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        let page_info = context
            .state
            .pages
            .iter_mut()
            .find(|info| info.id == oper.id)
            .ok_or_else(|| expected("error-page-not-found"))?;

        page_info.total_unit_count = oper.counters.total;

        page_info.translated_unit_count = oper.counters.translated;

        page_info.proofread_unit_count = oper.counters.proofread;

        page_info.updated_at = now();

        accept(())
    }
}

impl<'a> Step<ShiftPageIndexesTemporary<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Move current page indexes out of the final nonnegative range.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ShiftPageIndexesTemporary<'a>,
    ) -> BaseRest<()> {
        //
        for page_info in context
            .state
            .pages
            .iter_mut()
            .filter(|page_info| page_info.chapter_id == oper.chapter_id)
        {
            page_info.index = usize::MAX - page_info.index;

            page_info.updated_at = now();
        }

        accept(())
    }
}

impl<'a> Step<ApplyPageManifest<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Apply retained index updates and new page inserts as one mock operation.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ApplyPageManifest<'a>,
    ) -> BaseRest<Vec<PageInfo>> {
        //
        let mut seen_ids = HashSet::with_capacity(oper.entries.len());

        let mut page_infos = Vec::with_capacity(oper.entries.len());

        for entry in oper.entries {
            //
            let is_unique = seen_ids.insert(entry.id.as_str());

            if let false = is_unique {
                //
                return Err(unrecoverable(
                    "page manifest contains a duplicate page id",
                ));
            }

            let existing_page_info = context
                .state
                .pages
                .iter_mut()
                .find(|page_info| page_info.id == entry.id);

            let page_info = match existing_page_info {
                //
                Some(page_info) => {
                    //
                    if let false = page_info.chapter_id == entry.chapter_id {
                        //
                        return Err(unrecoverable(
                            "page manifest cannot move a page between chapters",
                        ));
                    }

                    page_info.index = entry.index;

                    page_info.updated_at = now();

                    page_info.clone()
                }

                None => {
                    //
                    let page_info = page_from_manifest_entry(entry);

                    context.state.pages.push(page_info.clone());

                    page_info
                }
            };

            page_infos.push(page_info);
        }

        accept(page_infos)
    }
}

impl<'a> Step<DeletePages<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeletePages<'a>,
    ) -> BaseRest<()> {
        //
        match oper {
            //
            // Internal implementation detail.
            DeletePages::Chapter { chapter_id } => {
                //
                // Internal implementation detail.
                let ids = context
                    .state
                    .pages
                    .iter()
                    .filter(|page_info| page_info.chapter_id == *chapter_id)
                    .map(|page_info| page_info.id.clone())
                    .collect::<Vec<_>>();

                context
                    .state
                    .units
                    .retain(|unit_info| !ids.contains(&unit_info.page_id));

                context
                    .state
                    .pages
                    .retain(|page_info| page_info.chapter_id != *chapter_id);

                accept(())
            }

            DeletePages::Ids { ids } => {
                //
                // Internal implementation detail.
                context
                    .state
                    .units
                    .retain(|unit_info| !ids.contains(&unit_info.page_id));

                context
                    .state
                    .pages
                    .retain(|page_info| !ids.contains(&page_info.id));

                accept(())
            }
        }
    }
}
