use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::chapter::ChapterInfo;
use crate::part::repo::oper::chapter::{
    AdjustChapterUnitCounters, CompleteChapterRawProvide, CreateChapter,
    DeleteChapter, FindPinnedChapterInfo, GetChapterInfo,
    GetChapterInfoExcluded, ListChapterInfos, ListChapterInfosExcluded,
    ListPinnedChapterInfos, LockChapters, ResetChapterRawProvide,
    SetChapterPageCounters, StartChapterStage, UnpinOtherChapters,
    UpdateChapter, UpdateChapterStage,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::chapter::step_impl::{
    adjust_unit_counters, complete_raw_provide, create, delete,
    find_pinned_info_by_comic_id, get_info_by_id, get_info_excluded,
    list_infos, list_infos_excluded, list_pinned_infos_by_comic_ids,
    lock_chapters, reset_raw_provide, set_page_counters, start_stage,
    unpin_others, update_info, update_stage,
};
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;

impl Run<GetChapterInfo<'_, '_>> for HybRepo {
    // Map failed query execution for chapter lookup into repository-level base error.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Execute chapter lookup through shared query submission so non-transactional callers
    // always reuse the same entry point and error path.
    async fn run(
        &self,
        oper: &GetChapterInfo<'_, '_>,
    ) -> BaseRest<ChapterInfo> {
        submit_query!(self.core, get_info_by_id, oper.id, oper.incls)
    }
}

impl Run<ListChapterInfos<'_>> for HybRepo {
    // Map list query failures to the common base error for callers.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Load chapter info list by specification, keeping read-only behavior at repository level.
    async fn run(
        &self,
        oper: &ListChapterInfos<'_>,
    ) -> BaseRest<Vec<ChapterInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl Run<FindPinnedChapterInfo<'_, '_>> for HybRepo {
    // Keep error handling consistent with other chapter lookup orchestrations.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Resolve at-most-one pinned chapter for a comic and include requested relations.
    async fn run(
        &self,
        oper: &FindPinnedChapterInfo<'_, '_>,
    ) -> BaseRest<Option<ChapterInfo>> {
        submit_query!(
            self.core,
            find_pinned_info_by_comic_id,
            oper.comic_id,
            oper.incls
        )
    }
}

impl Run<ListPinnedChapterInfos<'_>> for HybRepo {
    // Normalize all error paths for pinned-chapter batch reads.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Collect pinned chapter info for multiple comics.
    async fn run(
        &self,
        oper: &ListPinnedChapterInfos<'_>,
    ) -> BaseRest<Vec<ChapterInfo>> {
        submit_query!(self.core, list_pinned_infos_by_comic_ids, oper.comic_ids)
    }
}

impl Run<StartChapterStage<'_>> for HybRepo {
    // Ensure start-stage transition failures keep the same base error surface.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Enter a chapter state transition request and return whether any row changed.
    async fn run(&self, oper: &StartChapterStage<'_>) -> BaseRest<bool> {
        submit_query!(self.core, start_stage, oper.id, oper.stage)
    }
}

impl Run<CompleteChapterRawProvide<'_>> for HybRepo {
    // Keep completed-raw-provide orchestration errors as shared base errors.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Mark a chapter as ready for downstream raw provide workflow.
    async fn run(
        &self,
        oper: &CompleteChapterRawProvide<'_>,
    ) -> BaseRest<bool> {
        submit_query!(self.core, complete_raw_provide, oper.id)
    }
}

impl Step<CompleteChapterRawProvide<'_>, RdbContext> for HybRepo {
    // Keep internal step errors aligned with repository-level error semantics.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Execute the same raw-provide completion query inside the open transaction.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CompleteChapterRawProvide<'_>,
    ) -> BaseRest<bool> {
        complete_raw_provide(context.conn(), oper.id).await
    }
}

impl Step<ResetChapterRawProvide<'_>, RdbContext> for HybRepo {
    // Preserve unified error typing for resetting raw-provide state in transaction scope.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Reset chapter raw provide flags when downstream callers need a clean retry state.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ResetChapterRawProvide<'_>,
    ) -> BaseRest<()> {
        reset_raw_provide(context.conn(), oper.id).await
    }
}

impl Step<GetChapterInfo<'_, '_>, RdbContext> for HybRepo {
    // Keep transaction-level branch consistent with orchestrator-level error behavior.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Read chapter detail in transaction and return hydrated model data.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetChapterInfo<'_, '_>,
    ) -> BaseRest<ChapterInfo> {
        get_info_by_id(context.conn(), oper.id, oper.incls).await
    }
}

impl Step<GetChapterInfoExcluded<'_, '_>, RdbContext> for HybRepo {
    // Align error type for read queries that intentionally exclude soft-deleted rows.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Fetch chapter info with exclusion rules applied on top of includes.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetChapterInfoExcluded<'_, '_>,
    ) -> BaseRest<ChapterInfo> {
        get_info_excluded(context.conn(), oper.id, oper.incls).await
    }
}

impl Step<ListChapterInfosExcluded<'_>, RdbContext> for HybRepo {
    // Keep error surface stable for transactional filtered list queries.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Return all chapter infos under exclusion mode for one comic context.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListChapterInfosExcluded<'_>,
    ) -> BaseRest<Vec<ChapterInfo>> {
        list_infos_excluded(context.conn(), oper.comic_id).await
    }
}

impl Step<LockChapters<'_>, RdbContext> for HybRepo {
    // Use the same shared error model for lock orchestration failures.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Lock all chapters under a comic for a transactional edit window.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &LockChapters<'_>,
    ) -> BaseRest<()> {
        lock_chapters(context.conn(), oper.comic_id).await
    }
}

impl Step<FindPinnedChapterInfo<'_, '_>, RdbContext> for HybRepo {
    // Keep transactional lookup failures equivalent to non-transactional ones.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Resolve pinned chapter for a comic inside context and preserve include filters.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &FindPinnedChapterInfo<'_, '_>,
    ) -> BaseRest<Option<ChapterInfo>> {
        find_pinned_info_by_comic_id(context.conn(), oper.comic_id, oper.incls)
            .await
    }
}

impl Step<CreateChapter<'_>, RdbContext> for HybRepo {
    // Use a consistent error type for chapter creation inside transaction.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Insert a new chapter record and return persisted chapter payload.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateChapter<'_>,
    ) -> BaseRest<ChapterInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl Step<UpdateChapter<'_>, RdbContext> for HybRepo {
    // Normalize update errors to base repository errors under transaction.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Apply mutable chapter fields and return only success/failure.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateChapter<'_>,
    ) -> BaseRest<()> {
        update_info(context.conn(), oper.update).await
    }
}

impl Step<UpdateChapterStage<'_>, RdbContext> for HybRepo {
    // Keep stage-update failures in the same error vocabulary as other step operations.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Move chapter lifecycle state atomically inside the open transaction.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateChapterStage<'_>,
    ) -> BaseRest<()> {
        update_stage(context.conn(), oper.update).await
    }
}

impl Step<SetChapterPageCounters<'_>, RdbContext> for HybRepo {
    // Normalize counter-write failures for transactional chapter metrics updates.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Persist page and unit counters used by progress and rendering logic.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &SetChapterPageCounters<'_>,
    ) -> BaseRest<()> {
        set_page_counters(
            context.conn(),
            oper.id,
            oper.page_count,
            oper.total_unit_count,
            oper.translated_unit_count,
            oper.proofread_unit_count,
        )
        .await
    }
}

impl Step<AdjustChapterUnitCounters<'_>, RdbContext> for HybRepo {
    // Keep delta-based unit-counter adjustments mapped to base errors.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Apply signed unit counter drift to a chapter while preserving previous totals.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &AdjustChapterUnitCounters<'_>,
    ) -> BaseRest<()> {
        adjust_unit_counters(context.conn(), oper.id, &oper.delta).await
    }
}

impl Step<UnpinOtherChapters<'_>, RdbContext> for HybRepo {
    // Keep unpinning failures aligned with other transaction-level chapter operations.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Clear previous pinned chapters for the comic, excluding current target.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UnpinOtherChapters<'_>,
    ) -> BaseRest<()> {
        unpin_others(context.conn(), oper.comic_id, oper.excluded_id).await
    }
}

impl Step<DeleteChapter<'_>, RdbContext> for HybRepo {
    // Preserve consistent error reporting for chapter deletion operations.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Remove chapter row and rely on transaction caller to coordinate dependent effects.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteChapter<'_>,
    ) -> BaseRest<()> {
        delete(context.conn(), oper.id).await
    }
}
