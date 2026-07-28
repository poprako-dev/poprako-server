use std::collections::HashMap;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::page::PageInfo;
use crate::model::write::page::PageImageReservation;
use crate::part::repo::oper::page::{
    ClearPageImagesForPublish, CreatePages, DeletePages, GetPageInfo,
    GetPageInfoExcluded, ListFirstPageInfos, ListPageInfos,
    ListPageInfosExcluded, MarkPageImageUploaded, ReservePageImage,
    SetPageImageUploaded, SetPageUnitCounters, ShiftPageIndexesTemporary,
    UpdatePageManifest,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::page::step_impl::{
    clear_images_for_publish, create_batch, delete_by_chapter_id,
    delete_by_ids, get_info_by_id, get_info_excluded,
    list_first_infos_by_chapter_ids, list_infos, list_infos_excluded,
    mark_image_uploaded, reserve_image, set_image_uploaded, set_unit_counters,
    shift_indexes_temporary, update_manifest,
};
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseRest, ExpectedVariant};

impl Run<GetPageInfo<'_>> for RdbRepo {
    // Use base error for page read orchestration through the query dispatcher.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Fetch one page by id via shared repository dispatch.
    async fn run(&self, oper: &GetPageInfo<'_>) -> BaseRest<PageInfo> {
        submit_query!(self.core, get_info_by_id, oper.id)
    }
}

impl Run<ListPageInfos<'_>> for RdbRepo {
    // Keep list query failures aligned with repository-level base error handling.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // List page infos for a chapter using the chapter id filter.
    async fn run(&self, oper: &ListPageInfos<'_>) -> BaseRest<Vec<PageInfo>> {
        submit_query!(self.core, list_infos, oper.chapter_id)
    }
}

impl Run<ListFirstPageInfos<'_>> for RdbRepo {
    // Return base error for first-page batched read path.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Preload first-page info for each requested chapter id in one query batch.
    async fn run(
        &self,
        oper: &ListFirstPageInfos<'_>,
    ) -> BaseRest<HashMap<String, PageInfo>> {
        submit_query!(
            self.core,
            list_first_infos_by_chapter_ids,
            oper.chapter_ids
        )
    }
}

impl Step<GetPageInfo<'_>, RdbContext> for RdbRepo {
    // Use base error for row-level page reads inside a running transaction.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Read one page record in context and convert DB row into `PageInfo`.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetPageInfo<'_>,
    ) -> BaseRest<PageInfo> {
        get_info_by_id(context.conn(), oper.id).await
    }
}

impl Step<ListPageInfos<'_>, RdbContext> for RdbRepo {
    // Reuse base error semantics for chapter page list operations in transactions.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Load all pages under a chapter id directly from the transactional connection.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListPageInfos<'_>,
    ) -> BaseRest<Vec<PageInfo>> {
        list_infos(context.conn(), oper.chapter_id).await
    }
}

impl Step<ListPageInfosExcluded<'_>, RdbContext> for RdbRepo {
    // Keep excluded-list query errors on the shared base error channel.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Read pages for a chapter while applying exclusion rules for deleted rows.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListPageInfosExcluded<'_>,
    ) -> BaseRest<Vec<PageInfo>> {
        list_infos_excluded(context.conn(), oper.chapter_id).await
    }
}

impl Step<CreatePages<'_>, RdbContext> for RdbRepo {
    // Preserve base error behavior for batch page creation inside transaction.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Insert multiple new page entries and return their canonicalized infos.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreatePages<'_>,
    ) -> BaseRest<Vec<PageInfo>> {
        create_batch(context.conn(), oper.entries).await
    }
}

impl Step<GetPageInfoExcluded<'_>, RdbContext> for RdbRepo {
    // Use repository base error for filtered read path with row exclusion.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Load page detail under excluded-read options and return mapped `PageInfo`.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetPageInfoExcluded<'_>,
    ) -> BaseRest<PageInfo> {
        get_info_excluded(context.conn(), oper.id).await
    }
}

impl Step<ReservePageImage<'_>, RdbContext> for RdbRepo {
    // Map reservation failures to repository base errors.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Reserve upload metadata for a page image and return upload reservation info.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ReservePageImage<'_>,
    ) -> BaseRest<PageImageReservation> {
        reserve_image(context.conn(), oper.id, oper.file_ext).await
    }
}

impl Step<MarkPageImageUploaded<'_>, RdbContext> for RdbRepo {
    // Keep mark-upload status updates in base error domain.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Mark image as uploaded with version guard checks to avoid stale updates.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &MarkPageImageUploaded<'_>,
    ) -> BaseRest<()> {
        mark_image_uploaded(
            context.conn(),
            &oper.repl.id,
            oper.repl.image_version,
            oper.repl.image_key.as_deref(),
        )
        .await
    }
}

impl Step<SetPageImageUploaded<'_>, RdbContext> for RdbRepo {
    // Convert set-image state failures into base repository errors.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Set uploaded flag and persisted key/version for a page image.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &SetPageImageUploaded<'_>,
    ) -> BaseRest<()> {
        set_image_uploaded(
            context.conn(),
            &oper.repl.id,
            oper.repl.image_version,
            oper.repl.image_key.as_deref().ok_or_else(|| {
                BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: "page image key is required".into(),
                }
            })?,
            oper.repl.is_image_uploaded,
        )
        .await
    }
}

impl Step<SetPageUnitCounters<'_>, RdbContext> for RdbRepo {
    // Keep counter update failures consistent for transaction call sites.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Apply counter synchronization payload to page-level aggregates.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &SetPageUnitCounters<'_>,
    ) -> BaseRest<()> {
        set_unit_counters(context.conn(), oper.id, oper.counters).await
    }
}

impl Step<ShiftPageIndexesTemporary<'_>, RdbContext> for RdbRepo {
    // Maintain base-error parity for temporary page index reordering.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Perform temporary page index shifts for chapter-level reindex workflows.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ShiftPageIndexesTemporary<'_>,
    ) -> BaseRest<()> {
        shift_indexes_temporary(context.conn(), oper.chapter_id).await
    }
}

impl Step<UpdatePageManifest<'_>, RdbContext> for RdbRepo {
    // Preserve consistent error mapping while updating page manifest metadata.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Update manifest content and return refreshed page info in transaction.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdatePageManifest<'_>,
    ) -> BaseRest<PageInfo> {
        update_manifest(context.conn(), oper.update).await
    }
}

impl Step<ClearPageImagesForPublish<'_>, RdbContext> for RdbRepo {
    // Return base errors for image clear operations executed at publish time.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Clear publish-related image fields for a chapter and return affected page ids.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ClearPageImagesForPublish<'_>,
    ) -> BaseRest<Vec<String>> {
        clear_images_for_publish(context.conn(), oper.chapter_id).await
    }
}

impl Step<DeletePages<'_>, RdbContext> for RdbRepo {
    // Keep delete error semantics on the shared repository error type.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Delete pages by chapter or explicit IDs within the active transaction.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeletePages<'_>,
    ) -> BaseRest<()> {
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
