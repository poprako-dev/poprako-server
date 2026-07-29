use std::collections::HashMap;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::page::{PageImageReservation, PageInfo};
use crate::part::repo::oper::page::{
    CreatePages, DeletePages, GetPageInfo, GetPageInfoExcluded,
    ListFirstPageInfos, ListPageInfos, MarkPageImageUploaded, ReservePageImage,
    SetPageUnitCounters,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::page::{
    create_batch, delete_by_chapter_id, get_info_by_id, get_info_excluded,
    list_all_infos_by_chapter_id, list_first_infos_by_chapter_ids,
    list_infos_by_chapter_id, mark_image_uploaded, reserve_image,
    set_unit_counters,
};
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult};

impl<'a> Run<GetPageInfo<'a>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &GetPageInfo<'a>) -> BaseResult<PageInfo> {
        submit_query!(self.core, get_info_by_id, oper.id)
    }
}

impl<'a> Run<ListPageInfos<'a>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &ListPageInfos<'a>) -> BaseResult<Vec<PageInfo>> {
        match oper {
            //
            ListPageInfos::Chapter {
                chapter_id,
                offset,
                limit,
            } => {
                submit_query!(
                    self.core,
                    list_infos_by_chapter_id,
                    chapter_id,
                    *offset,
                    *limit
                )
            }

            ListPageInfos::AllChapter { chapter_id } => {
                submit_query!(
                    self.core,
                    list_all_infos_by_chapter_id,
                    chapter_id
                )
            }
        }
    }
}

impl<'a> Run<ListFirstPageInfos<'a>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListFirstPageInfos<'a>,
    ) -> BaseResult<HashMap<String, PageInfo>> {
        submit_query!(
            self.core,
            list_first_infos_by_chapter_ids,
            oper.chapter_ids
        )
    }
}

impl<'a> Step<GetPageInfo<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetPageInfo<'a>,
    ) -> BaseResult<PageInfo> {
        get_info_by_id(context.conn(), oper.id).await
    }
}

impl<'a> Step<ListPageInfos<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListPageInfos<'a>,
    ) -> BaseResult<Vec<PageInfo>> {
        match oper {
            //
            ListPageInfos::Chapter {
                chapter_id,
                offset,
                limit,
            } => {
                list_infos_by_chapter_id(
                    context.conn(),
                    chapter_id,
                    *offset,
                    *limit,
                )
                .await
            }

            ListPageInfos::AllChapter { chapter_id } => {
                list_all_infos_by_chapter_id(context.conn(), chapter_id).await
            }
        }
    }
}

impl<'a> Step<CreatePages<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreatePages<'a>,
    ) -> BaseResult<Vec<PageInfo>> {
        create_batch(context.conn(), oper.entries).await
    }
}

impl<'a> Step<GetPageInfoExcluded<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetPageInfoExcluded<'a>,
    ) -> BaseResult<PageInfo> {
        get_info_excluded(context.conn(), oper.id).await
    }
}

impl<'a> Step<ReservePageImage<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ReservePageImage<'a>,
    ) -> BaseResult<PageImageReservation> {
        reserve_image(context.conn(), oper.id, oper.file_ext).await
    }
}

impl<'a> Step<MarkPageImageUploaded<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &MarkPageImageUploaded<'a>,
    ) -> BaseResult<()> {
        mark_image_uploaded(context.conn(), oper.id, oper.image_version).await
    }
}

impl<'a> Step<SetPageUnitCounters<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &SetPageUnitCounters<'a>,
    ) -> BaseResult<()> {
        set_unit_counters(context.conn(), oper.id, oper.counters).await
    }
}

impl<'a> Step<DeletePages<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeletePages<'a>,
    ) -> BaseResult<()> {
        match oper {
            DeletePages::Chapter { chapter_id } => {
                delete_by_chapter_id(context.conn(), chapter_id).await
            }
        }
    }
}
