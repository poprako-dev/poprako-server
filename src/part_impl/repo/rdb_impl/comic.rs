// Stage-based comic filtering.
mod stage_filter;
// Comic step implementations.
mod step_impl;

/// Comic RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use poprako_orchestra::{AtLeast, Level, Run, Step};
use tracing::instrument;

use crate::model::read::proj::comic::ComicInfo;
use crate::model::write::comic::ComicCoverReservation;
use crate::part::nucl::RepeatableRead;
use crate::part::repo::oper::comic::{
    AllocComicChapterIndex, CreateComic, DeleteComic, GetComicInfo,
    GetComicInfoExcluded, ListComicInfos, ListComicInfosExcluded,
    MarkComicCoverUploaded, ReserveComicCover, TouchComicLastActive,
    UpdateComic, UpdateComicChapterCount,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::comic::step_impl::{
    create, delete, get_info_by_id, get_info_excluded, incr_chapter_next_index,
    list_infos, list_infos_excluded, mark_cover_uploaded, reserve_cover,
    touch_last_active, update_chapter_count, update_info,
};
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;

impl Run<GetComicInfo<'_, '_>> for HybRepo {
    // Maps the `GetComicInfo` repository operation to non-transactional execution.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Fetches one comic through `submit_query!` and applies caller-defined includes.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &GetComicInfo<'_, '_>) -> BaseRest<ComicInfo> {
        submit_query!(self.core, get_info_by_id, oper.id, oper.incls)
    }
}

impl Run<ListComicInfos<'_>> for HybRepo {
    // Maps the `ListComicInfos` repository operation to non-transactional execution.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Loads matching comics and returns the list view for the requested spec.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &ListComicInfos<'_>) -> BaseRest<Vec<ComicInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl Run<UpdateComic<'_>> for HybRepo {
    // Maps the `UpdateComic` repository operation to non-transactional execution.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Writes the provided comic updates using the step-level `update_info` flow.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &UpdateComic<'_>) -> BaseRest<()> {
        submit_query!(self.core, update_info, oper.update)
    }
}

impl Run<MarkComicCoverUploaded<'_>> for HybRepo {
    // Maps the `MarkComicCoverUploaded` repository operation to non-transactional execution.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Persists cover upload state (version/key/flag) and returns no payload.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &MarkComicCoverUploaded<'_>) -> BaseRest<()> {
        //
        submit_query!(
            self.core,
            mark_cover_uploaded,
            oper.id,
            oper.cover_version,
            oper.cover_key,
            oper.cover_uploaded
        )
    }
}

impl<L> Step<GetComicInfo<'_, '_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Resolves a single comic record inside an existing DB transaction context.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Loads one comic with requested includes by delegating to the step helper.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetComicInfo<'_, '_>,
    ) -> BaseRest<ComicInfo> {
        get_info_by_id(context.conn(), oper.id, oper.incls).await
    }
}

impl<L> Step<ListComicInfos<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Resolves a comic list inside an existing DB transaction context.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Applies the list spec in the transaction and returns matching comic infos.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListComicInfos<'_>,
    ) -> BaseRest<Vec<ComicInfo>> {
        list_infos(context.conn(), oper.spec).await
    }
}

impl<L> Step<GetComicInfoExcluded<'_, '_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Resolves one comic with excluded include payload inside a transaction.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Loads the comic while excluding non-essential relation expansion.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetComicInfoExcluded<'_, '_>,
    ) -> BaseRest<ComicInfo> {
        get_info_excluded(context.conn(), oper.id, oper.incls).await
    }
}

impl<L> Step<ListComicInfosExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Resolves a filtered excluded-comic list inside a transaction.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Applies excluded-list spec within transaction scope.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListComicInfosExcluded<'_>,
    ) -> BaseRest<Vec<ComicInfo>> {
        list_infos_excluded(context.conn(), oper.spec).await
    }
}

impl<L> Step<CreateComic<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Creates one comic inside an active transaction context.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Inserts the comic entry payload and returns the persisted comic info.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreateComic<'_>,
    ) -> BaseRest<ComicInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<L> Step<ReserveComicCover<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Reserves a comic cover upload slot inside an active transaction.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Creates reservation metadata for cover upload and returns claim details.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ReserveComicCover<'_>,
    ) -> BaseRest<ComicCoverReservation> {
        //
        reserve_cover(context.conn(), oper.id, oper.image_hash, oper.image_ext)
            .await
    }
}

impl<L> Step<MarkComicCoverUploaded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Marks cover upload state inside an active transaction.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Writes uploaded-cover state and persists metadata changes for the comic.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &MarkComicCoverUploaded<'_>,
    ) -> BaseRest<()> {
        //
        mark_cover_uploaded(
            context.conn(),
            oper.id,
            oper.cover_version,
            oper.cover_key,
            oper.cover_uploaded,
        )
        .await
    }
}

impl<L> Step<DeleteComic<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Deletes one comic inside an active transaction context.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Removes the comic record identified by id and returns after completion.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeleteComic<'_>,
    ) -> BaseRest<()> {
        delete(context.conn(), oper.id).await
    }
}

impl<L> Step<AllocComicChapterIndex<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Allocates the next chapter index inside an active transaction context.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Increments and returns the next chapter index for the comic.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &AllocComicChapterIndex<'_>,
    ) -> BaseRest<i32> {
        incr_chapter_next_index(context.conn(), oper.id).await
    }
}

impl<L> Step<UpdateComicChapterCount<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Updates chapter-count totals inside an active transaction context.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Applies a chapter-count delta to keep denormalized counters synchronized.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdateComicChapterCount<'_>,
    ) -> BaseRest<()> {
        update_chapter_count(context.conn(), oper.id, oper.delta).await
    }
}

impl<L> Step<TouchComicLastActive<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Touches last-active timestamp inside an active transaction context.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Updates comic activity marker so last-access timing stays fresh.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &TouchComicLastActive<'_>,
    ) -> BaseRest<()> {
        touch_last_active(context.conn(), oper.id).await
    }
}
