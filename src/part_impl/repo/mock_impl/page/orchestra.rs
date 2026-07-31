use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::complex::page::PageComplex;
use crate::model::read::proj::page::PageInfo;
use crate::model::write::page::PageImageReservation;
use crate::part::repo::oper::page::{
    ClearPageImagesForPublish, CreatePages, DeletePages, GetPageInfo,
    GetPageInfoExcluded, ListFirstPageInfos, ListPageInfos,
    ListPageInfosExcluded, MarkPageImageUploaded, ReservePageImage,
    SetPageImageUploaded, SetPageUnitCounters, ShiftPageIndexesTemporary,
    UpdatePageManifest,
};
use crate::part_impl::repo::mock_impl::page::{
    get_page_by_id, list_first_pages, list_infos, page_from_entry,
};
use crate::part_impl::repo::mock_impl::{Mock, MockContext, expected, now};
use crate::result::{BaseError, BaseRest, accept};

impl<'a> Run<GetPageInfo<'a>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &GetPageInfo<'a>) -> BaseRest<PageInfo> {
        //
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        get_page_by_id(&state, oper.id)
    }
}
impl<'a> Run<ListPageInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &ListPageInfos<'a>) -> BaseRest<Vec<PageInfo>> {
        //
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(list_infos(&state, oper.chapter_id))
    }
}

impl<'a> Run<ListFirstPageInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListFirstPageInfos<'a>,
    ) -> BaseRest<Vec<PageInfo>> {
        //
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(list_first_pages(&state, oper.chapter_ids))
    }
}
impl<'a> Step<GetPageInfo<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetPageInfo<'a>,
    ) -> BaseRest<PageInfo> {
        get_page_by_id(&context.state, oper.id)
    }
}
impl<'a> Step<ListPageInfos<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListPageInfos<'a>,
    ) -> BaseRest<Vec<PageInfo>> {
        accept(list_infos(&context.state, oper.chapter_id))
    }
}

impl<'a> Step<ListPageInfosExcluded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListPageInfosExcluded<'a>,
    ) -> BaseRest<Vec<PageInfo>> {
        accept(list_infos(&context.state, oper.chapter_id))
    }
}
impl<'a> Step<CreatePages<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreatePages<'a>,
    ) -> BaseRest<Vec<PageInfo>> {
        //
        // Internal implementation detail.
        if oper.entries.iter().any(|page_entry| {
            context
                .state
                .pages
                .iter()
                .any(|page_info| page_info.id == page_entry.id)
        }) {
            return Err(expected("error-already-exists"));
        }

        let infos =
            oper.entries.iter().map(page_from_entry).collect::<Vec<_>>();

        context.state.pages.extend(infos.clone());

        accept(infos)
    }
}
impl<'a> Step<GetPageInfoExcluded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetPageInfoExcluded<'a>,
    ) -> BaseRest<PageInfo> {
        get_page_by_id(&context.state, oper.id)
    }
}
impl<'a> Step<ReservePageImage<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ReservePageImage<'a>,
    ) -> BaseRest<PageImageReservation> {
        //
        // Internal implementation detail.
        let page_info = context
            .state
            .pages
            .iter_mut()
            .find(|info| info.id == oper.id)
            .ok_or_else(|| expected("error-page-not-found"))?;

        let prev_object_key = page_info.image_key.take();

        page_info.image_version += 1;

        page_info.is_image_uploaded = false;

        let object_key = PageComplex::gen_image_key(
            &page_info.chapter_id,
            oper.id,
            page_info.image_version,
            oper.file_ext,
        );

        page_info.image_key = Some(object_key.clone());

        page_info.updated_at = now();

        accept(PageImageReservation {
            object_key,
            prev_object_key,
            image_version: page_info.image_version,
        })
    }
}
impl<'a> Step<MarkPageImageUploaded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &MarkPageImageUploaded<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        let page_info = context
            .state
            .pages
            .iter_mut()
            .find(|info| info.id == oper.repl.id)
            .ok_or_else(|| expected("error-page-not-found"))?;

        if page_info.image_version != oper.repl.image_version
            || oper.repl.image_key.as_deref().is_some_and(|image_key| {
                page_info.image_key.as_deref() != Some(image_key)
            })
        {
            return Err(expected("error-stale-page-image-upload"));
        }

        page_info.is_image_uploaded = true;

        page_info.updated_at = now();

        accept(())
    }
}
impl<'a> Step<SetPageImageUploaded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &SetPageImageUploaded<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        let page_info = context
            .state
            .pages
            .iter_mut()
            .find(|info| info.id == oper.repl.id)
            .ok_or_else(|| expected("error-page-not-found"))?;

        if page_info.image_version != oper.repl.image_version
            || page_info.image_key.as_deref() != oper.repl.image_key.as_deref()
        {
            return Err(expected("error-stale-page-image-upload"));
        }

        page_info.is_image_uploaded = oper.repl.is_image_uploaded;

        page_info.updated_at = now();

        accept(())
    }
}
impl<'a> Step<SetPageUnitCounters<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &SetPageUnitCounters<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        let page_info = context
            .state
            .pages
            .iter_mut()
            .find(|info| info.id == oper.id)
            .ok_or_else(|| expected("error-page-not-found"))?;

        page_info.total_unit_count = oper.counters.total_unit_count;

        page_info.translated_unit_count = oper.counters.translated_unit_count;

        page_info.proofread_unit_count = oper.counters.proofread_unit_count;

        page_info.updated_at = now();

        accept(())
    }
}

impl<'a> Step<ShiftPageIndexesTemporary<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ShiftPageIndexesTemporary<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        for page_info in context.state.pages.iter_mut().filter(|page_info| {
            page_info.chapter_id == oper.chapter_id && page_info.index >= 0
        }) {
            //
            // Internal implementation detail.
            page_info.index = -page_info.index - 1;

            page_info.updated_at = now();
        }

        accept(())
    }
}

impl<'a> Step<UpdatePageManifest<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdatePageManifest<'a>,
    ) -> BaseRest<PageInfo> {
        //
        // Internal implementation detail.
        let page_info = context
            .state
            .pages
            .iter_mut()
            .find(|page_info| page_info.id == oper.update.id)
            .ok_or_else(|| expected("error-page-not-found"))?;

        page_info.index = oper.update.index;

        page_info.image_key = oper.update.image_key.clone();

        page_info.is_image_uploaded = oper.update.is_image_uploaded;

        page_info.image_version = oper.update.image_version;

        page_info.image_hash = oper.update.image_hash.clone();

        page_info.image_ext = oper.update.image_ext;

        page_info.updated_at = now();

        accept(page_info.clone())
    }
}

impl<'a> Step<ClearPageImagesForPublish<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ClearPageImagesForPublish<'a>,
    ) -> BaseRest<Vec<String>> {
        //
        // Internal implementation detail.
        let mut object_keys = Vec::new();

        for page_info in context
            .state
            .pages
            .iter_mut()
            .filter(|page_info| page_info.chapter_id == oper.chapter_id)
        {
            if let Some(object_key) = page_info.image_key.take() {
                object_keys.push(object_key);
            }

            page_info.is_image_uploaded = false;

            page_info.image_version = page_info
                .image_version
                .checked_add(1)
                .ok_or_else(|| BaseError::Unrecoverable {
                    message:
                        "[ClearPageImagesForPublish] image version overflow"
                            .into(),
                })?;

            page_info.updated_at = now();
        }

        accept(object_keys)
    }
}
impl<'a> Step<DeletePages<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeletePages<'a>,
    ) -> BaseRest<()> {
        match oper {
            //
            // Internal implementation detail.
            DeletePages::Chapter { chapter_id } => {
                //
                // Internal implementation detail.
                let ids = context
                    .state
                    .pages
                    .iter()
                    .filter(|page_info| page_info.chapter_id == *chapter_id)
                    .map(|page_info| page_info.id.clone())
                    .collect::<Vec<_>>();

                context
                    .state
                    .units
                    .retain(|unit_info| !ids.contains(&unit_info.page_id));

                context
                    .state
                    .pages
                    .retain(|page_info| page_info.chapter_id != *chapter_id);

                accept(())
            }

            DeletePages::Ids { ids } => {
                //
                // Internal implementation detail.
                context
                    .state
                    .units
                    .retain(|unit_info| !ids.contains(&unit_info.page_id));

                context
                    .state
                    .pages
                    .retain(|page_info| !ids.contains(&page_info.id));

                accept(())
            }
        }
    }
}
