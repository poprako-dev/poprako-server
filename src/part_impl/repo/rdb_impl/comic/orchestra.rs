use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::comic::{ComicCoverReservation, ComicInfo};
use crate::part::repo::oper::comic::{
    AllocComicChapterIndex, CreateComic, DeleteComic, GetComicInfo,
    GetComicInfoExcluded, ListComicInfos, ListComicInfosExcluded,
    MarkComicCoverUploaded, ReserveComicCover, TouchComicLastActive,
    UpdateComic, UpdateComicChapterCount,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::comic::step_impl::{
    create, delete, get_info_by_id, get_info_excluded, incr_chapter_next_index,
    list_infos, list_infos_excluded, mark_cover_uploaded, reserve_cover,
    touch_last_active, update_chapter_count, update_info,
};
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult};

impl Run<GetComicInfo<'_, '_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &GetComicInfo<'_, '_>) -> BaseResult<ComicInfo> {
        submit_query!(self.core, get_info_by_id, oper.id, oper.incls)
    }
}

impl Run<ListComicInfos<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListComicInfos<'_>,
    ) -> BaseResult<Vec<ComicInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl Run<UpdateComic<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &UpdateComic<'_>) -> BaseResult<()> {
        submit_query!(self.core, update_info, oper.update)
    }
}

impl Run<MarkComicCoverUploaded<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &MarkComicCoverUploaded<'_>) -> BaseResult<()> {
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

impl Step<GetComicInfo<'_, '_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetComicInfo<'_, '_>,
    ) -> BaseResult<ComicInfo> {
        get_info_by_id(context.conn(), oper.id, oper.incls).await
    }
}

impl Step<ListComicInfos<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListComicInfos<'_>,
    ) -> BaseResult<Vec<ComicInfo>> {
        list_infos(context.conn(), oper.spec).await
    }
}

impl Step<GetComicInfoExcluded<'_, '_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetComicInfoExcluded<'_, '_>,
    ) -> BaseResult<ComicInfo> {
        get_info_excluded(context.conn(), oper.id, oper.incls).await
    }
}

impl Step<ListComicInfosExcluded<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListComicInfosExcluded<'_>,
    ) -> BaseResult<Vec<ComicInfo>> {
        list_infos_excluded(context.conn(), oper.spec).await
    }
}

impl Step<CreateComic<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateComic<'_>,
    ) -> BaseResult<ComicInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl Step<ReserveComicCover<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ReserveComicCover<'_>,
    ) -> BaseResult<ComicCoverReservation> {
        reserve_cover(context.conn(), oper.id, oper.image_hash, oper.image_ext)
            .await
    }
}

impl Step<MarkComicCoverUploaded<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &MarkComicCoverUploaded<'_>,
    ) -> BaseResult<()> {
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

impl Step<DeleteComic<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteComic<'_>,
    ) -> BaseResult<()> {
        delete(context.conn(), oper.id).await
    }
}

impl Step<AllocComicChapterIndex<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &AllocComicChapterIndex<'_>,
    ) -> BaseResult<i32> {
        incr_chapter_next_index(context.conn(), oper.id).await
    }
}

impl Step<UpdateComicChapterCount<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateComicChapterCount<'_>,
    ) -> BaseResult<()> {
        update_chapter_count(context.conn(), oper.id, oper.delta).await
    }
}

impl Step<TouchComicLastActive<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &TouchComicLastActive<'_>,
    ) -> BaseResult<()> {
        touch_last_active(context.conn(), oper.id).await
    }
}
