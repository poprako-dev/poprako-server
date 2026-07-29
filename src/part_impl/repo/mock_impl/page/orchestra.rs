use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::complex::page::PageComplex;
use crate::model::page::{PageImageReservation, PageInfo};
use crate::part::repo::oper::page::{
    CreatePages, DeletePages, GetPageInfo, GetPageInfoExcluded,
    ListFirstPageInfos, ListPageInfos, MarkPageImageUploaded, ReservePageImage,
    SetPageUnitCounters,
};
use crate::part_impl::repo::mock_impl::page::{
    get_page_by_id, list_all_pages, list_first_pages, list_pages,
    page_from_entry,
};
use crate::part_impl::repo::mock_impl::{Mock, MockContext, expected, now};
use crate::result::{RegularError, RegularResult};

impl<'a> Run<GetPageInfo<'a>> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &GetPageInfo<'a>) -> RegularResult<PageInfo> {
        //
        let state = self.state.lock().unwrap();

        get_page_by_id(&state, oper.id)
    }
}
impl<'a> Run<ListPageInfos<'a>> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListPageInfos<'a>,
    ) -> RegularResult<Vec<PageInfo>> {
        //
        let state = self.state.lock().unwrap();

        match oper {
            //
            ListPageInfos::Chapter {
                chapter_id,
                offset,
                limit,
            } => Ok(list_pages(&state, chapter_id, *offset, *limit)),

            ListPageInfos::AllChapter { chapter_id } => {
                Ok(list_all_pages(&state, chapter_id))
            }
        }
    }
}

impl<'a> Run<ListFirstPageInfos<'a>> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListFirstPageInfos<'a>,
    ) -> RegularResult<std::collections::HashMap<String, PageInfo>> {
        //
        let state = self.state.lock().unwrap();

        Ok(list_first_pages(&state, oper.chapter_ids))
    }
}
impl<'a> Step<GetPageInfo<'a>, MockContext> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetPageInfo<'a>,
    ) -> RegularResult<PageInfo> {
        get_page_by_id(&context.state, oper.id)
    }
}
impl<'a> Step<ListPageInfos<'a>, MockContext> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListPageInfos<'a>,
    ) -> RegularResult<Vec<PageInfo>> {
        match oper {
            //
            ListPageInfos::Chapter {
                chapter_id,
                offset,
                limit,
            } => Ok(list_pages(&context.state, chapter_id, *offset, *limit)),

            ListPageInfos::AllChapter { chapter_id } => {
                Ok(list_all_pages(&context.state, chapter_id))
            }
        }
    }
}
impl<'a> Step<CreatePages<'a>, MockContext> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreatePages<'a>,
    ) -> RegularResult<Vec<PageInfo>> {
        //
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

        Ok(infos)
    }
}
impl<'a> Step<GetPageInfoExcluded<'a>, MockContext> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetPageInfoExcluded<'a>,
    ) -> RegularResult<PageInfo> {
        get_page_by_id(&context.state, oper.id)
    }
}
impl<'a> Step<ReservePageImage<'a>, MockContext> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ReservePageImage<'a>,
    ) -> RegularResult<PageImageReservation> {
        //
        let page_info = context
            .state
            .pages
            .iter_mut()
            .find(|info| info.id == oper.id)
            .ok_or_else(|| expected("error-page-not-found"))?;

        let prev_object_key = page_info.image_key.take();

        page_info.image_version += 1;

        page_info.image_uploaded = false;

        let object_key = PageComplex::gen_image_key(
            &page_info.chapter_id,
            oper.id,
            page_info.image_version,
            oper.file_ext,
        );

        page_info.image_key = Some(object_key.clone());

        page_info.updated_at = now();

        Ok(PageImageReservation {
            object_key,
            prev_object_key,
            image_version: page_info.image_version,
        })
    }
}
impl<'a> Step<MarkPageImageUploaded<'a>, MockContext> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &MarkPageImageUploaded<'a>,
    ) -> RegularResult<()> {
        //
        let page_info = context
            .state
            .pages
            .iter_mut()
            .find(|info| info.id == oper.id)
            .ok_or_else(|| expected("error-page-not-found"))?;

        if page_info.image_version != oper.image_version {
            return Err(expected("error-stale-page-image-upload"));
        }

        page_info.image_uploaded = true;

        page_info.updated_at = now();

        Ok(())
    }
}
impl<'a> Step<SetPageUnitCounters<'a>, MockContext> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &SetPageUnitCounters<'a>,
    ) -> RegularResult<()> {
        //
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

        Ok(())
    }
}
impl<'a> Step<DeletePages<'a>, MockContext> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeletePages<'a>,
    ) -> RegularResult<()> {
        match oper {
            DeletePages::Chapter { chapter_id } => {
                //
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

                Ok(())
            }
        }
    }
}
