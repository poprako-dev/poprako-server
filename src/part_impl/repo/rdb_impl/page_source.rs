//! RDB-backed page repository.

// Transaction-scoped page operations.
mod step_impl;

/// source page RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel::upsert::excluded;
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::{RdbConn, RdbCore};

use crate::model::read::proj::page_source::{
    PageSourceInfo, PageSourceRawIdentInfo, PageSourceUnitDiffStats,
    PageSourceUnitFlaggedStats, PageSourceUnitScope,
};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page_source::{
    ApplyPageSourceManifest, DeletePageSources, GetPageSourceInfo,
    GetPageSourceInfoExcluded, GetPageSourceUnitScope,
    GetPageSourceUnitScopeExcluded, ListFirstPageSourceInfos,
    ListPageSourceInfos, ListPageSourceInfosExcluded,
    ListPageSourceRawIdentInfos, ListPageSourceUnitDiffStats,
    ListPageSourceUnitFlaggedStats, SetPageSourceUnitCountMetrics,
    ShiftPageSourceIndexesTemporary, UpdatePageSourceRawIdents,
};
use crate::part_impl::repo::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::page_source::{
    PageSourceRawIdentInfoRecord, PageSourceRawIdentItem,
};
use crate::part_impl::repo::rdb_impl::page_source::step_impl::{
    apply_manifest, delete_by_chapter_id, delete_by_ids, get_info_by_id,
    get_info_excluded, get_unit_scope, get_unit_scope_excluded,
    list_first_infos_by_chapter_ids, list_infos, list_infos_excluded,
    list_unit_diff_stats, list_unit_flagged_stats, set_unit_counts,
    shift_indexes_temporary,
};
use crate::part_impl::repo::rdb_impl::schema::t_page_source_raw_ident;
use crate::result::{BaseError, BaseRest};
use crate::shape::write::page_source::PageSourceRawIdentsReplShape;
use crate::shared::RdbContext;
use crate::shared::result::diesel as map_diesel;

// Implements list raw ident infos.
#[instrument(level = "info", skip_all)]
async fn list_raw_ident_infos(
    core: &RdbCore,
    page_source_ids: &[&str],
) -> BaseRest<Vec<PageSourceRawIdentInfo>> {
    //
    if page_source_ids.is_empty() {
        return accept!(Vec::new());
    }

    let mut conn = core.get().await?;

    let records = t_page_source_raw_ident::table
        .filter(
            t_page_source_raw_ident::f_page_source_id.eq_any(page_source_ids),
        )
        .select(PageSourceRawIdentInfoRecord::as_select())
        .load::<PageSourceRawIdentInfoRecord>(&mut *conn)
        .await
        .map_err(map_diesel)?;

    accept!(
        records
            .into_iter()
            .map(PageSourceRawIdentInfo::from)
            .collect()
    )
}

// Implements update raw idents.
#[instrument(level = "info", skip_all)]
async fn update_raw_idents(
    conn: &mut RdbConn,
    repl: &PageSourceRawIdentsReplShape<'_>,
) -> BaseRest<()> {
    //
    let unnamed_ids = repl
        .idents
        .iter()
        .filter(|(_, raw_ident)| raw_ident.is_none())
        .map(|(page_source_id, _)| *page_source_id)
        .collect::<Vec<_>>();

    let items = repl
        .idents
        .iter()
        .filter_map(|(page_source_id, raw_ident)| {
            //
            Some(PageSourceRawIdentItem {
                f_page_source_id: page_source_id,
                f_raw_ident: (*raw_ident)?,
            })
        })
        .collect::<Vec<_>>();

    if !unnamed_ids.is_empty() {
        //
        diesel::delete(t_page_source_raw_ident::table.filter(
            t_page_source_raw_ident::f_page_source_id.eq_any(&unnamed_ids),
        ))
        .execute(conn)
        .await
        .map_err(map_diesel)?;
    }

    if !items.is_empty() {
        //
        diesel::insert_into(t_page_source_raw_ident::table)
            .values(&items)
            .on_conflict(t_page_source_raw_ident::f_page_source_id)
            .do_update()
            .set((
                t_page_source_raw_ident::f_raw_ident
                    .eq(excluded(t_page_source_raw_ident::f_raw_ident)),
                t_page_source_raw_ident::f_updated_at
                    .eq(OffsetDateTime::now_utc()),
            ))
            .execute(conn)
            .await
            .map_err(map_diesel)?;
    }

    accept!(())
}

impl Run<GetPageSourceInfo<'_>> for RdbRepo {
    // Use base error for page read orchestration through the query dispatcher.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Fetch one page by id via shared repository dispatch.
    async fn run(
        &self,
        oper: &GetPageSourceInfo<'_>,
    ) -> BaseRest<PageSourceInfo> {
        submit_query!(self.rdb_core, get_info_by_id, oper.id)
    }
}

impl Run<GetPageSourceUnitScope<'_>> for RdbRepo {
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Loads the minimal PageSource scope needed by Unit operations.
    async fn run(
        &self,
        oper: &GetPageSourceUnitScope<'_>,
    ) -> BaseRest<PageSourceUnitScope> {
        submit_query!(self.rdb_core, get_unit_scope, oper.id)
    }
}

impl Run<ListPageSourceInfos<'_>> for RdbRepo {
    // Keep list query failures aligned with repository-level base error handling.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // List page infos for a chapter using the chapter id filter.
    async fn run(
        &self,
        oper: &ListPageSourceInfos<'_>,
    ) -> BaseRest<Vec<PageSourceInfo>> {
        submit_query!(self.rdb_core, list_infos, oper.chapter_id)
    }
}

impl Run<ListFirstPageSourceInfos<'_>> for RdbRepo {
    // Return base error for first-page batched read path.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Preload first-page info for each requested chapter id in one query batch.
    async fn run(
        &self,
        oper: &ListFirstPageSourceInfos<'_>,
    ) -> BaseRest<Vec<PageSourceInfo>> {
        //
        submit_query!(
            self.rdb_core,
            list_first_infos_by_chapter_ids,
            oper.chapter_ids
        )
    }
}

impl Run<ListPageSourceUnitDiffStats<'_>> for RdbRepo {
    // Error type for the Chapter proofread-diff PageSource query.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    //
    // Lists matching PageSource text statistics in stable Chapter PageSource order.
    async fn run(
        &self,
        oper: &ListPageSourceUnitDiffStats<'_>,
    ) -> BaseRest<Vec<PageSourceUnitDiffStats>> {
        submit_query!(self.rdb_core, list_unit_diff_stats, oper.chapter_id)
    }
}

impl Run<ListPageSourceUnitFlaggedStats<'_>> for RdbRepo {
    // Error type for the Chapter flagged Unit PageSource query.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    //
    // Lists matching PageSource flagged Unit statistics in stable Chapter PageSource order.
    async fn run(
        &self,
        oper: &ListPageSourceUnitFlaggedStats<'_>,
    ) -> BaseRest<Vec<PageSourceUnitFlaggedStats>> {
        submit_query!(self.rdb_core, list_unit_flagged_stats, oper.chapter_id)
    }
}

impl<L> Step<GetPageSourceInfo<'_>, RdbContext<L>> for RdbRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use base error for record-level page reads inside a running transaction.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Read one page record in context and convert it into `PageSourceInfo`.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetPageSourceInfo<'_>,
    ) -> BaseRest<PageSourceInfo> {
        get_info_by_id(context.conn(), oper.id).await
    }
}

impl<L> Step<GetPageSourceUnitScope<'_>, RdbContext<L>> for RdbRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Declares the transaction isolation level required for this read.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Loads the minimal PageSource scope from the active transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetPageSourceUnitScope<'_>,
    ) -> BaseRest<PageSourceUnitScope> {
        get_unit_scope(context.conn(), oper.id).await
    }
}

impl<L> Step<ListPageSourceInfos<'_>, RdbContext<L>> for RdbRepo
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
        oper: &ListPageSourceInfos<'_>,
    ) -> BaseRest<Vec<PageSourceInfo>> {
        list_infos(context.conn(), oper.chapter_id).await
    }
}

impl<L> Step<ListPageSourceInfosExcluded<'_>, RdbContext<L>> for RdbRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep excluded-list query errors on the shared base error channel.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Read pages for a chapter while applying exclusion rules for deleted records.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListPageSourceInfosExcluded<'_>,
    ) -> BaseRest<Vec<PageSourceInfo>> {
        list_infos_excluded(context.conn(), oper.chapter_id).await
    }
}

impl<L> Step<GetPageSourceInfoExcluded<'_>, RdbContext<L>> for RdbRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use repository base error for filtered read path with record exclusion.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Load page detail under excluded-read options and return mapped `PageSourceInfo`.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetPageSourceInfoExcluded<'_>,
    ) -> BaseRest<PageSourceInfo> {
        get_info_excluded(context.conn(), oper.id).await
    }
}

impl<L> Step<GetPageSourceUnitScopeExcluded<'_>, RdbContext<L>> for RdbRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Declares the transaction isolation level required for this locked read.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Locks and loads the minimal PageSource scope used by Unit edits.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetPageSourceUnitScopeExcluded<'_>,
    ) -> BaseRest<PageSourceUnitScope> {
        get_unit_scope_excluded(context.conn(), oper.id).await
    }
}

impl<L> Step<SetPageSourceUnitCountMetrics<'_>, RdbContext<L>> for RdbRepo
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
        oper: &SetPageSourceUnitCountMetrics<'_>,
    ) -> BaseRest<()> {
        set_unit_counts(context.conn(), oper.id, oper.count_metrics).await
    }
}

impl<L> Step<ShiftPageSourceIndexesTemporary<'_>, RdbContext<L>> for RdbRepo
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
        oper: &ShiftPageSourceIndexesTemporary<'_>,
    ) -> BaseRest<()> {
        shift_indexes_temporary(context.conn(), oper.chapter_id).await
    }
}

impl<L> Step<ApplyPageSourceManifest<'_>, RdbContext<L>> for RdbRepo
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
        oper: &ApplyPageSourceManifest<'_>,
    ) -> BaseRest<Vec<PageSourceInfo>> {
        apply_manifest(context.conn(), oper.entries).await
    }
}

impl<L> Step<DeletePageSources<'_>, RdbContext<L>> for RdbRepo
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
        oper: &DeletePageSources<'_>,
    ) -> BaseRest<()> {
        //
        match oper {
            //
            DeletePageSources::Chapter { chapter_id } => {
                delete_by_chapter_id(context.conn(), chapter_id).await
            }

            DeletePageSources::Ids { ids } => {
                delete_by_ids(context.conn(), ids).await
            }
        }
    }
}

impl Run<ListPageSourceRawIdentInfos<'_>> for RdbRepo {
    // Shared application error type.
    type Error = BaseError;

    // Loads only requested page associations in one query.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &ListPageSourceRawIdentInfos<'_>,
    ) -> BaseRest<Vec<PageSourceRawIdentInfo>> {
        list_raw_ident_infos(&self.rdb_core, oper.page_source_ids).await
    }
}

impl<L> Step<UpdatePageSourceRawIdents<'_>, RdbContext<L>> for RdbRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Allocation holds the owning chapter and page locks.
    type Level = ReptRead;

    // Shared application error type.
    type Error = BaseError;

    // Replaces optional source filenames in at most two statements.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdatePageSourceRawIdents<'_>,
    ) -> BaseRest<()> {
        update_raw_idents(context.conn(), oper.repl).await
    }
}
