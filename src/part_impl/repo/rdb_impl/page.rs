//! RDB-backed page repository.

// Transaction-scoped page operations.
mod step_impl;

/// Page RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use poprako_orchestra::{AtLeast, Level, Run, Step};
use tracing::instrument;

use crate::model::read::proj::page::PageInfo;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page::{
    ApplyPageManifest, CreatePages, DeletePages, GetPageInfo,
    GetPageInfoExcluded, ListFirstPageInfos, ListPageInfos,
    ListPageInfosExcluded, SetPageUnitCounters, ShiftPageIndexesTemporary,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::page::step_impl::{
    apply_manifest, create_batch, delete_by_chapter_id, delete_by_ids,
    get_info_by_id, get_info_excluded, list_first_infos_by_chapter_ids,
    list_infos, list_infos_excluded, set_unit_counters,
    shift_indexes_temporary,
};
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;

impl Run<GetPageInfo<'_>> for HybRepo {
    // Use base error for page read orchestration through the query dispatcher.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Fetch one page by id via shared repository dispatch.
    async fn run(&self, oper: &GetPageInfo<'_>) -> BaseRest<PageInfo> {
        submit_query!(self.core, get_info_by_id, oper.id)
    }
}

impl Run<ListPageInfos<'_>> for HybRepo {
    // Keep list query failures aligned with repository-level base error handling.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // List page infos for a chapter using the chapter id filter.
    async fn run(&self, oper: &ListPageInfos<'_>) -> BaseRest<Vec<PageInfo>> {
        submit_query!(self.core, list_infos, oper.chapter_id)
    }
}

impl Run<ListFirstPageInfos<'_>> for HybRepo {
    // Return base error for first-page batched read path.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Preload first-page info for each requested chapter id in one query batch.
    async fn run(
        &self,
        oper: &ListFirstPageInfos<'_>,
    ) -> BaseRest<Vec<PageInfo>> {
        //
        submit_query!(
            self.core,
            list_first_infos_by_chapter_ids,
            oper.chapter_ids
        )
    }
}

impl<L> Step<GetPageInfo<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use base error for row-level page reads inside a running transaction.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Read one page record in context and convert DB row into `PageInfo`.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetPageInfo<'_>,
    ) -> BaseRest<PageInfo> {
        get_info_by_id(context.conn(), oper.id).await
    }
}

impl<L> Step<ListPageInfos<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Reuse base error semantics for chapter page list operations in transactions.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Load all pages under a chapter id directly from the transactional connection.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListPageInfos<'_>,
    ) -> BaseRest<Vec<PageInfo>> {
        list_infos(context.conn(), oper.chapter_id).await
    }
}

impl<L> Step<ListPageInfosExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep excluded-list query errors on the shared base error channel.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Read pages for a chapter while applying exclusion rules for deleted rows.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListPageInfosExcluded<'_>,
    ) -> BaseRest<Vec<PageInfo>> {
        list_infos_excluded(context.conn(), oper.chapter_id).await
    }
}

impl<L> Step<CreatePages<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Preserve base error behavior for batch page creation inside transaction.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Insert multiple new page entries and return their canonicalized infos.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreatePages<'_>,
    ) -> BaseRest<Vec<PageInfo>> {
        create_batch(context.conn(), oper.entries).await
    }
}

impl<L> Step<GetPageInfoExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use repository base error for filtered read path with row exclusion.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Load page detail under excluded-read options and return mapped `PageInfo`.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetPageInfoExcluded<'_>,
    ) -> BaseRest<PageInfo> {
        get_info_excluded(context.conn(), oper.id).await
    }
}

impl<L> Step<SetPageUnitCounters<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep counter update failures consistent for transaction call sites.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Apply counter synchronization payload to page-level aggregates.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &SetPageUnitCounters<'_>,
    ) -> BaseRest<()> {
        set_unit_counters(context.conn(), oper.id, oper.counters).await
    }
}

impl<L> Step<ShiftPageIndexesTemporary<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Maintain base-error parity for temporary page index reordering.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Move current indexes aside before the manifest batch applies final indexes.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ShiftPageIndexesTemporary<'_>,
    ) -> BaseRest<()> {
        shift_indexes_temporary(context.conn(), oper.chapter_id).await
    }
}

impl<L> Step<ApplyPageManifest<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Maintain base-error parity for page manifest writes.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Apply all final page identities and indexes with one typed batch upsert.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ApplyPageManifest<'_>,
    ) -> BaseRest<Vec<PageInfo>> {
        apply_manifest(context.conn(), oper.entries).await
    }
}

impl<L> Step<DeletePages<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep delete error semantics on the shared repository error type.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Delete pages by chapter or explicit IDs within the active transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeletePages<'_>,
    ) -> BaseRest<()> {
        //
        match oper {
            //
            DeletePages::Chapter { chapter_id } => {
                delete_by_chapter_id(context.conn(), chapter_id).await
            }

            DeletePages::Ids { ids } => {
                delete_by_ids(context.conn(), ids).await
            }
        }
    }
}
