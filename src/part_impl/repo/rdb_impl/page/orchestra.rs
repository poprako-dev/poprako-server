use std::collections::HashMap;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::page::{PageImageReservation, PageInfo};
use crate::part::repo::oper::page::{
    ClearPageImagesForPublish, CreatePages, DeletePages, GetPageInfo,
    GetPageInfoExcluded, ListFirstPageInfos, ListPageInfos,
    ListPageInfosExcluded, MarkPageImageUploaded, ReservePageImage,
    SetPageUnitCounters, ShiftPageIndexesTemporary, UpdatePageManifest,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::page::step_impl::{
    clear_images_for_publish, create_batch, delete_by_chapter_id,
    delete_by_ids, get_info_by_id, get_info_excluded,
    list_all_infos_by_chapter_id, list_all_infos_excluded_by_chapter_id,
    list_first_infos_by_chapter_ids, list_infos_by_chapter_id,
    mark_image_uploaded, reserve_image, set_unit_counters,
    shift_indexes_temporary, update_manifest,
};
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult};

impl Run<GetPageInfo<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &GetPageInfo<'_>) -> BaseResult<PageInfo> {
        submit_query!(self.core, get_info_by_id, oper.id)
    }
}

impl Run<ListPageInfos<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &ListPageInfos<'_>) -> BaseResult<Vec<PageInfo>> {
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

impl Run<ListFirstPageInfos<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListFirstPageInfos<'_>,
    ) -> BaseResult<HashMap<String, PageInfo>> {
        submit_query!(
            self.core,
            list_first_infos_by_chapter_ids,
            oper.chapter_ids
        )
    }
}

impl Step<GetPageInfo<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetPageInfo<'_>,
    ) -> BaseResult<PageInfo> {
        get_info_by_id(context.conn(), oper.id).await
    }
}

impl Step<ListPageInfos<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListPageInfos<'_>,
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

impl Step<ListPageInfosExcluded<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListPageInfosExcluded<'_>,
    ) -> BaseResult<Vec<PageInfo>> {
        list_all_infos_excluded_by_chapter_id(context.conn(), oper.chapter_id)
            .await
    }
}

impl Step<CreatePages<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreatePages<'_>,
    ) -> BaseResult<Vec<PageInfo>> {
        create_batch(context.conn(), oper.entries).await
    }
}

impl Step<GetPageInfoExcluded<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetPageInfoExcluded<'_>,
    ) -> BaseResult<PageInfo> {
        get_info_excluded(context.conn(), oper.id).await
    }
}

impl Step<ReservePageImage<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ReservePageImage<'_>,
    ) -> BaseResult<PageImageReservation> {
        reserve_image(context.conn(), oper.id, oper.file_ext).await
    }
}

impl Step<MarkPageImageUploaded<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &MarkPageImageUploaded<'_>,
    ) -> BaseResult<()> {
        mark_image_uploaded(
            context.conn(),
            oper.id,
            oper.image_version,
            oper.image_key,
        )
        .await
    }
}

impl Step<SetPageUnitCounters<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &SetPageUnitCounters<'_>,
    ) -> BaseResult<()> {
        set_unit_counters(context.conn(), oper.id, oper.counters).await
    }
}

impl Step<ShiftPageIndexesTemporary<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ShiftPageIndexesTemporary<'_>,
    ) -> BaseResult<()> {
        shift_indexes_temporary(context.conn(), oper.chapter_id).await
    }
}

impl Step<UpdatePageManifest<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdatePageManifest<'_>,
    ) -> BaseResult<PageInfo> {
        update_manifest(context.conn(), oper.update).await
    }
}

impl Step<ClearPageImagesForPublish<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ClearPageImagesForPublish<'_>,
    ) -> BaseResult<Vec<String>> {
        clear_images_for_publish(context.conn(), oper.chapter_id).await
    }
}

impl Step<DeletePages<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeletePages<'_>,
    ) -> BaseResult<()> {
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
