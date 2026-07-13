use poprako_orchestra::{Run, Step};

use crate::model::comic::ComicCoverReservation;
use crate::model::comic::ComicInfo;
use crate::part::repo::oper::comic::{
    AllocateComicChapterIndex, CreateComic, DeleteComic, GetComicInfo,
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
use crate::result::{RegularError, RegularResult};

impl<'a, 'b> Run<GetComicInfo<'a, 'b>> for RdbRepo {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &GetComicInfo<'a, 'b>,
    ) -> RegularResult<ComicInfo> {
        submit_query!(self.core, get_info_by_id, oper.id, oper.incls)
    }
}

impl<'a> Run<ListComicInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &ListComicInfos<'a>,
    ) -> RegularResult<Vec<ComicInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl<'a> Run<UpdateComic<'a>> for RdbRepo {
    type Error = RegularError;

    async fn run(&self, oper: &UpdateComic<'a>) -> RegularResult<()> {
        submit_query!(self.core, update_info, oper.update)
    }
}

impl<'a> Run<MarkComicCoverUploaded<'a>> for RdbRepo {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &MarkComicCoverUploaded<'a>,
    ) -> RegularResult<()> {
        submit_query!(
            self.core,
            mark_cover_uploaded,
            oper.id,
            oper.cover_version
        )
    }
}

impl<'a, 'b> Step<GetComicInfo<'a, 'b>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetComicInfo<'a, 'b>,
    ) -> RegularResult<ComicInfo> {
        get_info_by_id(context.conn(), oper.id, oper.incls).await
    }
}

impl<'a> Step<ListComicInfos<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListComicInfos<'a>,
    ) -> RegularResult<Vec<ComicInfo>> {
        list_infos(context.conn(), oper.spec).await
    }
}

impl<'a, 'b> Step<GetComicInfoExcluded<'a, 'b>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetComicInfoExcluded<'a, 'b>,
    ) -> RegularResult<ComicInfo> {
        get_info_excluded(context.conn(), oper.id, oper.incls).await
    }
}

impl<'a> Step<ListComicInfosExcluded<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListComicInfosExcluded<'a>,
    ) -> RegularResult<Vec<ComicInfo>> {
        list_infos_excluded(context.conn(), oper.spec).await
    }
}

impl<'a> Step<CreateComic<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateComic<'a>,
    ) -> RegularResult<ComicInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<'a> Step<ReserveComicCover<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ReserveComicCover<'a>,
    ) -> RegularResult<ComicCoverReservation> {
        reserve_cover(context.conn(), oper.id, oper.file_extension).await
    }
}

impl<'a> Step<MarkComicCoverUploaded<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &MarkComicCoverUploaded<'a>,
    ) -> RegularResult<()> {
        mark_cover_uploaded(context.conn(), oper.id, oper.cover_version).await
    }
}

impl<'a> Step<DeleteComic<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteComic<'a>,
    ) -> RegularResult<()> {
        delete(context.conn(), oper.id).await
    }
}

impl<'a> Step<AllocateComicChapterIndex<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &AllocateComicChapterIndex<'a>,
    ) -> RegularResult<i32> {
        incr_chapter_next_index(context.conn(), oper.id).await
    }
}

impl<'a> Step<UpdateComicChapterCount<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateComicChapterCount<'a>,
    ) -> RegularResult<()> {
        update_chapter_count(context.conn(), oper.id, oper.delta).await
    }
}

impl<'a> Step<TouchComicLastActive<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &TouchComicLastActive<'a>,
    ) -> RegularResult<()> {
        touch_last_active(context.conn(), oper.id).await
    }
}
