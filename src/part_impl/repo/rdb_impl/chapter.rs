//! RDB-backed chapter repository.

// Transaction-scoped chapter operations.
mod step_impl;

/// Chapter RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use poprako_orchestra::{AtLeast, Level, Run, Step};
use tracing::instrument;

use crate::model::read::proj::chapter::{ChapterInfo, ChapterUnitEditScope};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::chapter::{
    AdjustChapterUnitCountDelta, CompleteChapterRawProvide, CreateChapter,
    FindPinnedChapterInfo, GetChapterInfo, GetChapterInfoExcluded,
    GetChapterUnitEditScopeExcluded, ListChapterInfos,
    ListChapterInfosExcluded, ListPinnedChapterInfos, LockChapters,
    SetChapterPageCountMetrics, StartChapterStage, UnpinOtherChapters,
    UpdateChapter, UpdateChapterStage,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::chapter::step_impl::{
    adjust_unit_counts, complete_raw_provide, create,
    find_pinned_info_by_comic_id, get_info_by_id, get_info_excluded,
    get_unit_edit_scope_excluded, list_infos, list_infos_excluded,
    list_pinned_infos_by_comic_ids, lock_chapters, set_page_counts,
    start_stage, unpin_others, update_info, update_stage,
};
use crate::part_impl::repo::rdb_impl::numeric::i32_from_usize;
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;

impl Run<GetChapterInfo<'_, '_>> for HybRepo {
    // Map failed query execution for chapter lookup into repository-level base error.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Execute chapter lookup through shared query submission so non-transactional callers
    // always reuse the same entry point and error path.
    async fn run(
        &self,
        oper: &GetChapterInfo<'_, '_>,
    ) -> BaseRest<ChapterInfo> {
        submit_query!(self.rdb_core, get_info_by_id, oper.id, oper.incls)
    }
}

impl Run<ListChapterInfos<'_>> for HybRepo {
    // Map list query failures to the common base error for callers.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Load chapter info list by specification, keeping read-only behavior at repository level.
    async fn run(
        &self,
        oper: &ListChapterInfos<'_>,
    ) -> BaseRest<Vec<ChapterInfo>> {
        submit_query!(self.rdb_core, list_infos, oper.spec)
    }
}

impl Run<FindPinnedChapterInfo<'_, '_>> for HybRepo {
    // Keep error handling consistent with other chapter lookup orchestrations.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Resolve at-most-one pinned chapter for a comic and include requested relations.
    async fn run(
        &self,
        oper: &FindPinnedChapterInfo<'_, '_>,
    ) -> BaseRest<Option<ChapterInfo>> {
        //
        submit_query!(
            self.rdb_core,
            find_pinned_info_by_comic_id,
            oper.comic_id,
            oper.incls
        )
    }
}

impl Run<ListPinnedChapterInfos<'_>> for HybRepo {
    // Normalize all error paths for pinned-chapter batch reads.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    //
    // Collect pinned chapter info for multiple comics.
    async fn run(
        &self,
        oper: &ListPinnedChapterInfos<'_>,
    ) -> BaseRest<Vec<ChapterInfo>> {
        //
        submit_query!(
            self.rdb_core,
            list_pinned_infos_by_comic_ids,
            oper.comic_ids
        )
    }
}

impl Run<StartChapterStage<'_>> for HybRepo {
    // Ensure start-stage transition failures keep the same base error surface.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Enter a chapter state transition request and return whether any row changed.
    async fn run(&self, oper: &StartChapterStage<'_>) -> BaseRest<bool> {
        submit_query!(self.rdb_core, start_stage, oper.id, oper.stage)
    }
}

impl Run<CompleteChapterRawProvide<'_>> for HybRepo {
    // Keep completed-raw-provide orchestration errors as shared base errors.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Mark a chapter as ready for downstream raw provide workflow.
    async fn run(
        &self,
        oper: &CompleteChapterRawProvide<'_>,
    ) -> BaseRest<bool> {
        submit_query!(self.rdb_core, complete_raw_provide, oper.id)
    }
}

impl<L> Step<StartChapterStage<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Declares the transaction isolation level required for this mutation.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Starts the requested pending stage inside the caller-owned transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &StartChapterStage<'_>,
    ) -> BaseRest<bool> {
        start_stage(context.conn(), oper.id, oper.stage).await
    }
}

impl<L> Step<CompleteChapterRawProvide<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep internal step errors aligned with repository-level error semantics.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Execute the same raw-provide completion query inside the open transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CompleteChapterRawProvide<'_>,
    ) -> BaseRest<bool> {
        complete_raw_provide(context.conn(), oper.id).await
    }
}

impl<L> Step<GetChapterInfo<'_, '_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep transaction-level branch consistent with orchestrator-level error behavior.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Read chapter detail in transaction and return hydrated model data.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetChapterInfo<'_, '_>,
    ) -> BaseRest<ChapterInfo> {
        get_info_by_id(context.conn(), oper.id, oper.incls).await
    }
}

impl<L> Step<GetChapterInfoExcluded<'_, '_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Align error type for read queries that intentionally exclude soft-deleted rows.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Fetch chapter info with exclusion rules applied on top of includes.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetChapterInfoExcluded<'_, '_>,
    ) -> BaseRest<ChapterInfo> {
        get_info_excluded(context.conn(), oper.id, oper.incls).await
    }
}

impl<L> Step<GetChapterUnitEditScopeExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Declares the transaction isolation level required for this locked read.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Locks and loads the minimal Chapter scope owning the requested Page.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetChapterUnitEditScopeExcluded<'_>,
    ) -> BaseRest<ChapterUnitEditScope> {
        get_unit_edit_scope_excluded(context.conn(), oper.page_id).await
    }
}

impl<L> Step<ListChapterInfosExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep error surface stable for transactional filtered list queries.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Return all chapter infos under exclusion mode for one comic context.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListChapterInfosExcluded<'_>,
    ) -> BaseRest<Vec<ChapterInfo>> {
        list_infos_excluded(context.conn(), oper.comic_id).await
    }
}

impl<L> Step<LockChapters<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use the same shared error model for lock orchestration failures.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Lock all chapters under a comic for a transactional edit window.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &LockChapters<'_>,
    ) -> BaseRest<()> {
        lock_chapters(context.conn(), oper.comic_id).await
    }
}

impl<L> Step<FindPinnedChapterInfo<'_, '_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep transactional lookup failures equivalent to non-transactional ones.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Resolve pinned chapter for a comic inside context and preserve include filters.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &FindPinnedChapterInfo<'_, '_>,
    ) -> BaseRest<Option<ChapterInfo>> {
        //
        find_pinned_info_by_comic_id(context.conn(), oper.comic_id, oper.incls)
            .await
    }
}

impl<L> Step<CreateChapter<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use a consistent error type for chapter creation inside transaction.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Insert a new chapter record and return persisted chapter payload.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreateChapter<'_>,
    ) -> BaseRest<ChapterInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<L> Step<UpdateChapter<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Normalize update errors to base repository errors under transaction.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Apply mutable chapter fields and return only success/failure.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdateChapter<'_>,
    ) -> BaseRest<()> {
        update_info(context.conn(), oper.update).await
    }
}

impl<L> Step<UpdateChapterStage<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep stage-update failures in the same error vocabulary as other step operations.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Move chapter lifecycle state atomically inside the open transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdateChapterStage<'_>,
    ) -> BaseRest<()> {
        update_stage(context.conn(), oper.update).await
    }
}

impl<L> Step<SetChapterPageCountMetrics<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Normalize counter-write failures for transactional chapter metrics updates.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Persist page and unit counters used by progress and rendering logic.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &SetChapterPageCountMetrics<'_>,
    ) -> BaseRest<()> {
        //
        set_page_counts(
            context.conn(),
            oper.id,
            i32_from_usize(oper.page_count, "t_chapter.f_page_count")?,
            i32_from_usize(
                oper.total_unit_count,
                "t_chapter.f_total_unit_count",
            )?,
            i32_from_usize(
                oper.translated_unit_count,
                "t_chapter.f_translated_unit_count",
            )?,
            i32_from_usize(
                oper.proofread_unit_count,
                "t_chapter.f_proofread_unit_count",
            )?,
        )
        .await
    }
}

impl<L> Step<AdjustChapterUnitCountDelta<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep delta-based unit-counter adjustments mapped to base errors.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Apply signed unit counter drift to a chapter while preserving previous totals.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &AdjustChapterUnitCountDelta<'_>,
    ) -> BaseRest<()> {
        adjust_unit_counts(context.conn(), oper.id, &oper.delta).await
    }
}

impl<L> Step<UnpinOtherChapters<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep unpinning failures aligned with other transaction-level chapter operations.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Clear previous pinned chapters for the comic, excluding current target.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UnpinOtherChapters<'_>,
    ) -> BaseRest<()> {
        unpin_others(context.conn(), oper.comic_id, oper.excluded_id).await
    }
}
