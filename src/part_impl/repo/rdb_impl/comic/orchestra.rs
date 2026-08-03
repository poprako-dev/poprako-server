use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::comic::ComicInfo;
use crate::model::write::comic::ComicCoverReservation;
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
    type Error = BaseError;

    // Fetches one comic through `submit_query!` and applies caller-defined includes.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &GetComicInfo<'_, '_>) -> BaseRest<ComicInfo> {
        submit_query!(self.core, get_info_by_id, oper.id, oper.incls)
    }
}

impl Run<ListComicInfos<'_>> for HybRepo {
    // Maps the `ListComicInfos` repository operation to non-transactional execution.
    type Error = BaseError;

    // Loads matching comics and returns the list view for the requested spec.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &ListComicInfos<'_>) -> BaseRest<Vec<ComicInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl Run<UpdateComic<'_>> for HybRepo {
    // Maps the `UpdateComic` repository operation to non-transactional execution.
    type Error = BaseError;

    // Writes the provided comic updates using the step-level `update_info` flow.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &UpdateComic<'_>) -> BaseRest<()> {
        submit_query!(self.core, update_info, oper.update)
    }
}

impl Run<MarkComicCoverUploaded<'_>> for HybRepo {
    // Maps the `MarkComicCoverUploaded` repository operation to non-transactional execution.
    type Error = BaseError;

    // Persists cover upload state (version/key/flag) and returns no payload.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &MarkComicCoverUploaded<'_>) -> BaseRest<()> {
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

impl Step<GetComicInfo<'_, '_>, RdbContext> for HybRepo {
    // Resolves a single comic record inside an existing DB transaction context.
    type Error = BaseError;

    // Loads one comic with requested includes by delegating to the step helper.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetComicInfo<'_, '_>,
    ) -> BaseRest<ComicInfo> {
        get_info_by_id(context.conn(), oper.id, oper.incls).await
    }
}

impl Step<ListComicInfos<'_>, RdbContext> for HybRepo {
    // Resolves a comic list inside an existing DB transaction context.
    type Error = BaseError;

    // Applies the list spec in the transaction and returns matching comic infos.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListComicInfos<'_>,
    ) -> BaseRest<Vec<ComicInfo>> {
        list_infos(context.conn(), oper.spec).await
    }
}

impl Step<GetComicInfoExcluded<'_, '_>, RdbContext> for HybRepo {
    // Resolves one comic with excluded include payload inside a transaction.
    type Error = BaseError;

    // Loads the comic while excluding non-essential relation expansion.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetComicInfoExcluded<'_, '_>,
    ) -> BaseRest<ComicInfo> {
        get_info_excluded(context.conn(), oper.id, oper.incls).await
    }
}

impl Step<ListComicInfosExcluded<'_>, RdbContext> for HybRepo {
    // Resolves a filtered excluded-comic list inside a transaction.
    type Error = BaseError;

    // Applies excluded-list spec within transaction scope.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListComicInfosExcluded<'_>,
    ) -> BaseRest<Vec<ComicInfo>> {
        list_infos_excluded(context.conn(), oper.spec).await
    }
}

impl Step<CreateComic<'_>, RdbContext> for HybRepo {
    // Creates one comic inside an active transaction context.
    type Error = BaseError;

    // Inserts the comic entry payload and returns the persisted comic info.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateComic<'_>,
    ) -> BaseRest<ComicInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl Step<ReserveComicCover<'_>, RdbContext> for HybRepo {
    // Reserves a comic cover upload slot inside an active transaction.
    type Error = BaseError;

    // Creates reservation metadata for cover upload and returns claim details.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ReserveComicCover<'_>,
    ) -> BaseRest<ComicCoverReservation> {
        reserve_cover(context.conn(), oper.id, oper.image_hash, oper.image_ext)
            .await
    }
}

impl Step<MarkComicCoverUploaded<'_>, RdbContext> for HybRepo {
    // Marks cover upload state inside an active transaction.
    type Error = BaseError;

    // Writes uploaded-cover state and persists metadata changes for the comic.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &MarkComicCoverUploaded<'_>,
    ) -> BaseRest<()> {
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

impl Step<DeleteComic<'_>, RdbContext> for HybRepo {
    // Deletes one comic inside an active transaction context.
    type Error = BaseError;

    // Removes the comic record identified by id and returns after completion.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteComic<'_>,
    ) -> BaseRest<()> {
        delete(context.conn(), oper.id).await
    }
}

impl Step<AllocComicChapterIndex<'_>, RdbContext> for HybRepo {
    // Allocates the next chapter index inside an active transaction context.
    type Error = BaseError;

    // Increments and returns the next chapter index for the comic.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &AllocComicChapterIndex<'_>,
    ) -> BaseRest<i32> {
        incr_chapter_next_index(context.conn(), oper.id).await
    }
}

impl Step<UpdateComicChapterCount<'_>, RdbContext> for HybRepo {
    // Updates chapter-count totals inside an active transaction context.
    type Error = BaseError;

    // Applies a chapter-count delta to keep denormalized counters synchronized.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateComicChapterCount<'_>,
    ) -> BaseRest<()> {
        update_chapter_count(context.conn(), oper.id, oper.delta).await
    }
}

impl Step<TouchComicLastActive<'_>, RdbContext> for HybRepo {
    // Touches last-active timestamp inside an active transaction context.
    type Error = BaseError;

    // Updates comic activity marker so last-access timing stays fresh.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &TouchComicLastActive<'_>,
    ) -> BaseRest<()> {
        touch_last_active(context.conn(), oper.id).await
    }
}
