//! RDB-backed page repository.

mod step_impl;

/// Page RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use poprako_orchestra::{AtLeast, Level, Run, Step};
use tracing::instrument;

use crate::model::read::proj::page::PageInfo;
use crate::model::write::page::PageImageReservation;
use crate::part::nucl::RepeatableRead;
use crate::part::repo::oper::page::{
    ClearPageImagesForPublish, CreatePages, DeletePages, GetPageInfo,
    GetPageInfoExcluded, ListFirstPageInfos, ListPageInfos,
    ListPageInfosExcluded, MarkPageImageUploaded, ReservePageImage,
    SetPageImageUploaded, SetPageUnitCounters, ShiftPageIndexesTemporary,
    UpdatePageManifest,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::page::step_impl::{
    clear_images_for_publish, create_batch, delete_by_chapter_id,
    delete_by_ids, get_info_by_id, get_info_excluded,
    list_first_infos_by_chapter_ids, list_infos, list_infos_excluded,
    mark_image_uploaded, reserve_image, set_image_uploaded, set_unit_counters,
    shift_indexes_temporary, update_manifest,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::shared::RdbContext;

impl Run<GetPageInfo<'_>> for HybRepo {
    // Use base error for page read orchestration through the query dispatcher.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Fetch one page by id via shared repository dispatch.
    async fn run(&self, oper: &GetPageInfo<'_>) -> BaseRest<PageInfo> {
        submit_query!(self.core, get_info_by_id, oper.id)
    }
}

impl Run<ListPageInfos<'_>> for HybRepo {
    // Keep list query failures aligned with repository-level base error handling.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // List page infos for a chapter using the chapter id filter.
    async fn run(&self, oper: &ListPageInfos<'_>) -> BaseRest<Vec<PageInfo>> {
        submit_query!(self.core, list_infos, oper.chapter_id)
    }
}

impl Run<ListFirstPageInfos<'_>> for HybRepo {
    // Return base error for first-page batched read path.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Preload first-page info for each requested chapter id in one query batch.
    async fn run(
        &self,
        oper: &ListFirstPageInfos<'_>,
    ) -> BaseRest<Vec<PageInfo>> {
        //
        submit_query!(
            self.core,
            list_first_infos_by_chapter_ids,
            oper.chapter_ids
        )
    }
}

impl<L> Step<GetPageInfo<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Use base error for row-level page reads inside a running transaction.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Read one page record in context and convert DB row into `PageInfo`.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetPageInfo<'_>,
    ) -> BaseRest<PageInfo> {
        get_info_by_id(context.conn(), oper.id).await
    }
}

impl<L> Step<ListPageInfos<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Reuse base error semantics for chapter page list operations in transactions.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Load all pages under a chapter id directly from the transactional connection.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListPageInfos<'_>,
    ) -> BaseRest<Vec<PageInfo>> {
        list_infos(context.conn(), oper.chapter_id).await
    }
}

impl<L> Step<ListPageInfosExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Keep excluded-list query errors on the shared base error channel.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Read pages for a chapter while applying exclusion rules for deleted rows.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListPageInfosExcluded<'_>,
    ) -> BaseRest<Vec<PageInfo>> {
        list_infos_excluded(context.conn(), oper.chapter_id).await
    }
}

impl<L> Step<CreatePages<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Preserve base error behavior for batch page creation inside transaction.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Insert multiple new page entries and return their canonicalized infos.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreatePages<'_>,
    ) -> BaseRest<Vec<PageInfo>> {
        create_batch(context.conn(), oper.entries).await
    }
}

impl<L> Step<GetPageInfoExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Use repository base error for filtered read path with row exclusion.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Load page detail under excluded-read options and return mapped `PageInfo`.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetPageInfoExcluded<'_>,
    ) -> BaseRest<PageInfo> {
        get_info_excluded(context.conn(), oper.id).await
    }
}

impl<L> Step<ReservePageImage<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Map reservation failures to repository base errors.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Reserve upload metadata for a page image and return upload reservation info.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ReservePageImage<'_>,
    ) -> BaseRest<PageImageReservation> {
        reserve_image(context.conn(), oper.id, oper.file_ext).await
    }
}

impl<L> Step<MarkPageImageUploaded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Keep mark-upload status updates in base error domain.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Mark image as uploaded with version guard checks to avoid stale updates.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &MarkPageImageUploaded<'_>,
    ) -> BaseRest<()> {
        //
        mark_image_uploaded(
            context.conn(),
            &oper.repl.id,
            oper.repl.image_version,
            oper.repl.image_key.as_deref(),
        )
        .await
    }
}

impl<L> Step<SetPageImageUploaded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Convert set-image state failures into base repository errors.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Set uploaded flag and persisted key/version for a page image.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &SetPageImageUploaded<'_>,
    ) -> BaseRest<()> {
        //
        set_image_uploaded(
            context.conn(),
            &oper.repl.id,
            oper.repl.image_version,
            oper.repl.image_key.as_deref().ok_or_else(|| {
                //
                let err_message = String::from("page image key is required");

                tracing::warn!(
                    error_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    page_id = %oper.repl.id,
                    image_version = oper.repl.image_version,
                    image_key_present = oper.repl.image_key.is_some(),
                    image_uploaded = oper.repl.is_image_uploaded,
                    stage = "set_image_uploaded",
                    "expected error: page image key is required",
                );

                BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                }
            })?,
            oper.repl.is_image_uploaded,
        )
        .await
    }
}

impl<L> Step<SetPageUnitCounters<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Keep counter update failures consistent for transaction call sites.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Apply counter synchronization payload to page-level aggregates.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &SetPageUnitCounters<'_>,
    ) -> BaseRest<()> {
        set_unit_counters(context.conn(), oper.id, oper.counters).await
    }
}

impl<L> Step<ShiftPageIndexesTemporary<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Maintain base-error parity for temporary page index reordering.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Perform temporary page index shifts for chapter-level reindex workflows.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ShiftPageIndexesTemporary<'_>,
    ) -> BaseRest<()> {
        shift_indexes_temporary(context.conn(), oper.chapter_id).await
    }
}

impl<L> Step<UpdatePageManifest<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Preserve consistent error mapping while updating page manifest metadata.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Update manifest content and return refreshed page info in transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdatePageManifest<'_>,
    ) -> BaseRest<PageInfo> {
        update_manifest(context.conn(), oper.update).await
    }
}

impl<L> Step<ClearPageImagesForPublish<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Return base errors for image clear operations executed at publish time.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Clear publish-related image fields for a chapter and return affected page ids.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ClearPageImagesForPublish<'_>,
    ) -> BaseRest<Vec<String>> {
        clear_images_for_publish(context.conn(), oper.chapter_id).await
    }
}

impl<L> Step<DeletePages<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<RepeatableRead>,
{
    // Keep delete error semantics on the shared repository error type.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Delete pages by chapter or explicit IDs within the active transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeletePages<'_>,
    ) -> BaseRest<()> {
        //
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
