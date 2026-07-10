//! Mock implementations of `PageRepo` and `PageRepoTransactional`.

use async_trait::async_trait;

use poprako_transactional::advance::Advance;

use crate::complex::page::PageComplex;
use crate::model::page::{PageForm, PageImageReservation, PageInfo};
use crate::part::repo::page::{PageRepo, PageRepoTransactional};
use crate::part::repo::step::page::{
    CreateBatch, DeleteByChapterId, GetInfoById, GetInfoExcluded,
    ListAllInfosByChapterId, ListInfosByChapterId, MarkImageUploaded,
    ReserveImage, SetUnitCounters,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, MockTransactional, expected, now,
};
use crate::result::{RegularError, RegularResult};

impl PageRepo<MockContext> for Mock {}

impl PageRepoTransactional<MockContext> for MockTransactional {}

fn get_page_by_id(state: &MockState, id: &str) -> RegularResult<PageInfo> {
    state
        .pages
        .iter()
        .find(|page_info| page_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-page-not-found"))
}

fn list_all_pages(state: &MockState, chapter_id: &str) -> Vec<PageInfo> {
    //
    let mut page_infos = state
        .pages
        .iter()
        .filter(|page_info| page_info.chapter_id == chapter_id)
        .cloned()
        .collect::<Vec<_>>();

    page_infos.sort_by_key(|left| left.index);

    page_infos
}

fn list_pages(
    state: &MockState,
    chapter_id: &str,
    offset: u64,
    limit: u64,
) -> Vec<PageInfo> {
    //
    let page_infos = list_all_pages(state, chapter_id);

    let offset = offset as usize;

    let limit = limit as usize;

    if offset >= page_infos.len() {
        return Vec::new();
    }

    let end = std::cmp::min(offset + limit, page_infos.len());

    page_infos[offset..end].to_vec()
}

fn page_from_form(form: &PageForm) -> PageInfo {
    //
    let time = now();

    PageInfo {
        id: form.id.clone(),
        chapter_id: form.chapter_id.clone(),
        index: form.index,
        image_key: form.image_key.clone(),
        image_uploaded: false,
        image_version: form.image_version,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for Mock {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &GetInfoById<'a>,
    ) -> Result<PageInfo, Self::Error> {
        let state = self.state.lock().unwrap();
        get_page_by_id(&state, step.id)
    }
}

#[async_trait]
impl<'a> Execute<ListInfosByChapterId<'a>> for Mock {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfosByChapterId<'a>,
    ) -> Result<Vec<PageInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        Ok(list_pages(&state, step.chapter_id, step.offset, step.limit))
    }
}

#[async_trait]
impl<'a> Execute<ListAllInfosByChapterId<'a>> for Mock {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListAllInfosByChapterId<'a>,
    ) -> Result<Vec<PageInfo>, Self::Error> {
        let state = self.state.lock().unwrap();

        Ok(list_all_pages(&state, step.chapter_id))
    }
}

#[async_trait]
impl<'a> Advance<GetInfoById<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoById<'a>,
    ) -> Result<PageInfo, Self::Error> {
        get_page_by_id(&context.state, step.id)
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoExcluded<'a>,
    ) -> Result<PageInfo, Self::Error> {
        get_page_by_id(&context.state, step.id)
    }
}

#[async_trait]
impl<'a> Advance<ListInfosByChapterId<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ListInfosByChapterId<'a>,
    ) -> Result<Vec<PageInfo>, Self::Error> {
        Ok(list_pages(
            &context.state,
            step.chapter_id,
            step.offset,
            step.limit,
        ))
    }
}

#[async_trait]
impl<'a> Advance<ListAllInfosByChapterId<'a>, MockContext>
    for MockTransactional
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ListAllInfosByChapterId<'a>,
    ) -> Result<Vec<PageInfo>, Self::Error> {
        Ok(list_all_pages(&context.state, step.chapter_id))
    }
}

#[async_trait]
impl<'a> Advance<CreateBatch<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &CreateBatch<'a>,
    ) -> Result<Vec<PageInfo>, Self::Error> {
        if step.forms.iter().any(|page_form| {
            context
                .state
                .pages
                .iter()
                .any(|page_info| page_info.id == page_form.id)
        }) {
            return Err(expected("error-already-exists"));
        }

        let page_infos =
            step.forms.iter().map(page_from_form).collect::<Vec<_>>();

        context.state.pages.extend(page_infos.clone());

        Ok(page_infos)
    }
}

#[async_trait]
impl<'a> Advance<ReserveImage<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ReserveImage<'a>,
    ) -> Result<PageImageReservation, Self::Error> {
        let page_info = context
            .state
            .pages
            .iter_mut()
            .find(|page_info| page_info.id == step.id)
            .ok_or_else(|| expected("error-page-not-found"))?;

        let image_version = page_info.image_version + 1;
        let object_key = PageComplex::gen_image_key(
            &page_info.chapter_id,
            step.id,
            image_version,
            step.file_ext,
        );
        let prev_object_key = page_info.image_key.clone();
        page_info.image_key = Some(object_key.clone());
        page_info.image_uploaded = false;
        page_info.image_version = image_version;
        page_info.updated_at = now();

        Ok(PageImageReservation {
            object_key,
            prev_object_key,
            image_version,
        })
    }
}

#[async_trait]
impl<'a> Advance<MarkImageUploaded<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &MarkImageUploaded<'a>,
    ) -> Result<(), Self::Error> {
        let page_info = context
            .state
            .pages
            .iter_mut()
            .find(|page_info| page_info.id == step.id)
            .ok_or_else(|| expected("error-page-not-found"))?;

        if page_info.image_version != step.image_version {
            return Err(expected("error-stale-page-image-upload"));
        }

        page_info.image_uploaded = true;
        page_info.updated_at = now();

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<SetUnitCounters<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &SetUnitCounters<'a>,
    ) -> Result<(), Self::Error> {
        let page_info = context
            .state
            .pages
            .iter_mut()
            .find(|page_info| page_info.id == step.id)
            .ok_or_else(|| expected("error-page-not-found"))?;

        page_info.total_unit_count = step.counters.total_unit_count;
        page_info.translated_unit_count = step.counters.translated_unit_count;
        page_info.proofread_unit_count = step.counters.proofread_unit_count;
        page_info.updated_at = now();

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<DeleteByChapterId<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &DeleteByChapterId<'a>,
    ) -> Result<(), Self::Error> {
        context
            .state
            .pages
            .retain(|page_info| page_info.chapter_id != step.chapter_id);
        let page_ids = context
            .state
            .pages
            .iter()
            .map(|page_info| page_info.id.clone())
            .collect::<Vec<_>>();
        context
            .state
            .units
            .retain(|unit_info| page_ids.contains(&unit_info.page_id));

        Ok(())
    }
}
