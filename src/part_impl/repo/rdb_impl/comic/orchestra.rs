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
use crate::part_impl::repo::rdb_impl::comic::{
    create, delete, get_info_by_id, get_info_excluded, incr_chapter_next_index,
    list_infos, list_infos_excluded, mark_cover_uploaded, reserve_cover,
    touch_last_active, update_chapter_count, update_info,
};
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult};

impl<'a, 'b> Run<GetComicInfo<'a, 'b>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &GetComicInfo<'a, 'b>) -> BaseResult<ComicInfo> {
        submit_query!(self.core, get_info_by_id, oper.id, oper.incls)
    }
}

impl<'a> Run<ListComicInfos<'a>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListComicInfos<'a>,
    ) -> BaseResult<Vec<ComicInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl<'a> Run<UpdateComic<'a>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &UpdateComic<'a>) -> BaseResult<()> {
        submit_query!(self.core, update_info, oper.update)
    }
}

impl<'a> Run<MarkComicCoverUploaded<'a>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &MarkComicCoverUploaded<'a>) -> BaseResult<()> {
        submit_query!(
            self.core,
            mark_cover_uploaded,
            oper.id,
            oper.cover_version
        )
    }
}

impl<'a, 'b> Step<GetComicInfo<'a, 'b>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetComicInfo<'a, 'b>,
    ) -> BaseResult<ComicInfo> {
        get_info_by_id(context.conn(), oper.id, oper.incls).await
    }
}

impl<'a> Step<ListComicInfos<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListComicInfos<'a>,
    ) -> BaseResult<Vec<ComicInfo>> {
        list_infos(context.conn(), oper.spec).await
    }
}

impl<'a, 'b> Step<GetComicInfoExcluded<'a, 'b>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetComicInfoExcluded<'a, 'b>,
    ) -> BaseResult<ComicInfo> {
        get_info_excluded(context.conn(), oper.id, oper.incls).await
    }
}

impl<'a> Step<ListComicInfosExcluded<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListComicInfosExcluded<'a>,
    ) -> BaseResult<Vec<ComicInfo>> {
        list_infos_excluded(context.conn(), oper.spec).await
    }
}

impl<'a> Step<CreateComic<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateComic<'a>,
    ) -> BaseResult<ComicInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<'a> Step<ReserveComicCover<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ReserveComicCover<'a>,
    ) -> BaseResult<ComicCoverReservation> {
        reserve_cover(context.conn(), oper.id, oper.file_extension).await
    }
}

impl<'a> Step<MarkComicCoverUploaded<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &MarkComicCoverUploaded<'a>,
    ) -> BaseResult<()> {
        mark_cover_uploaded(context.conn(), oper.id, oper.cover_version).await
    }
}

impl<'a> Step<DeleteComic<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteComic<'a>,
    ) -> BaseResult<()> {
        delete(context.conn(), oper.id).await
    }
}

impl<'a> Step<AllocComicChapterIndex<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &AllocComicChapterIndex<'a>,
    ) -> BaseResult<i32> {
        incr_chapter_next_index(context.conn(), oper.id).await
    }
}

impl<'a> Step<UpdateComicChapterCount<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateComicChapterCount<'a>,
    ) -> BaseResult<()> {
        update_chapter_count(context.conn(), oper.id, oper.delta).await
    }
}

impl<'a> Step<TouchComicLastActive<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &TouchComicLastActive<'a>,
    ) -> BaseResult<()> {
        touch_last_active(context.conn(), oper.id).await
    }
}
