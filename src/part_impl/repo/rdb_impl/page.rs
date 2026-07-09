//! RDB-backed page repository.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::complex::page::PageComplex;
use crate::model::page::{PageForm, PageImageReservation, PageInfo};
use crate::model::unit::UnitCounters;
use crate::part::repo::page::{PageRepo, PageRepoTransactional};
use crate::part::repo::step::page::{
    CreateBatch, DeleteByChapterId, GetInfoById, GetInfoExcluded,
    ListAllInfosByChapterId, ListInfosByChapterId, MarkImageUploaded,
    ReserveImage, SetUnitCounters,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::repo::rdb_impl::entity::page::{
    PageAspect, PageEntry, PageRow,
};
use crate::part_impl::repo::rdb_impl::{RdbRepo, RdbRepoTransactional};
use crate::result::{RegularError, RegularResult};

use crate::part_impl::repo::rdb_impl::schema::t_page::dsl::*;
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{f_page_id as unit_f_page_id, t_unit};

impl PageRepo<RdbContext> for RdbRepo {}

impl PageRepoTransactional<RdbContext> for RdbRepoTransactional {}

async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<PageInfo> {
    //
    let row: PageRow = t_page
        .filter(f_id.eq(id))
        .select(PageRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-page-not-found"))?;

    Ok(row.into())
}

async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<PageInfo> {
    //
    let row: PageRow = t_page
        .filter(f_id.eq(id))
        .select(PageRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-page-not-found"))?;

    Ok(row.into())
}

async fn list_infos_by_chapter_id(
    conn: &mut RdbConn,
    chapter_id: &str,
    offset: u64,
    limit: u64,
) -> RegularResult<Vec<PageInfo>> {
    //
    let rows: Vec<PageRow> = t_page
        .filter(f_chapter_id.eq(chapter_id))
        .select(PageRow::as_select())
        .order_by(f_index.asc())
        .offset(offset as i64)
        .limit(limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

async fn list_all_infos_by_chapter_id(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> RegularResult<Vec<PageInfo>> {
    //
    let rows: Vec<PageRow> = t_page
        .filter(f_chapter_id.eq(chapter_id))
        .select(PageRow::as_select())
        .order_by(f_index.asc())
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

async fn create_batch(
    conn: &mut RdbConn,
    forms: &[PageForm],
) -> RegularResult<Vec<PageInfo>> {
    //
    let entries: Vec<PageEntry> = forms.iter().map(PageEntry::from).collect();

    let rows: Vec<PageRow> = diesel::insert_into(t_page)
        .values(&entries)
        .returning(PageRow::as_returning())
        .get_results(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

async fn reserve_image(
    conn: &mut RdbConn,
    id: &str,
    file_ext: &str,
) -> RegularResult<PageImageReservation> {
    //
    let now = OffsetDateTime::now_utc();

    let (chapter_id, prev_key, new_version): (String, Option<String>, i64) =
        diesel::update(t_page.filter(f_id.eq(id)))
            .set((
                f_image_key.eq::<Option<&str>>(None),
                f_image_uploaded.eq(false),
                f_image_version.eq(f_image_version + 1),
                f_updated_at.eq(now),
            ))
            .returning((f_chapter_id, f_image_key, f_image_version))
            .get_result(conn)
            .await
            .map_err(diesel)?;

    let object_key =
        PageComplex::gen_image_key(&chapter_id, id, new_version, file_ext);

    let aspect = PageAspect::new(now).image_key(Some(&object_key));

    diesel::update(t_page.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(PageImageReservation {
        object_key,
        prev_object_key: prev_key,
        image_version: new_version,
    })
}

async fn mark_image_uploaded(
    conn: &mut RdbConn,
    id: &str,
    image_version: i64,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let affected = diesel::update(
        t_page
            .filter(f_id.eq(id))
            .filter(f_image_version.eq(image_version)),
    )
    .set((f_image_uploaded.eq(true), f_updated_at.eq(now)))
    .execute(conn)
    .await
    .map_err(diesel)?;

    if affected == 0 {
        return Err(expected("error-stale-page-image-upload"));
    }

    Ok(())
}

async fn set_unit_counters(
    conn: &mut RdbConn,
    id: &str,
    counters: UnitCounters,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = PageAspect::new(now)
        .total_unit_count(counters.total_unit_count)
        .translated_unit_count(counters.translated_unit_count)
        .proofread_unit_count(counters.proofread_unit_count);

    diesel::update(t_page.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn delete_by_chapter_id(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> RegularResult<()> {
    //
    let page_ids: Vec<String> = t_page
        .filter(f_chapter_id.eq(chapter_id))
        .select(f_id)
        .load(conn)
        .await
        .map_err(diesel)?;

    if !page_ids.is_empty() {
        diesel::delete(t_unit.filter(unit_f_page_id.eq_any(&page_ids)))
            .execute(conn)
            .await
            .map_err(diesel)?;
    }

    diesel::delete(t_page.filter(f_chapter_id.eq(chapter_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &GetInfoById<'a>) -> RegularResult<PageInfo> {
        submit_query!(self.core, get_info_by_id, step.id)
    }
}

#[async_trait]
impl<'a> Execute<ListInfosByChapterId<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfosByChapterId<'a>,
    ) -> RegularResult<Vec<PageInfo>> {
        submit_query!(
            self.core,
            list_infos_by_chapter_id,
            step.chapter_id,
            step.offset,
            step.limit
        )
    }
}

#[async_trait]
impl<'a> Execute<ListAllInfosByChapterId<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListAllInfosByChapterId<'a>,
    ) -> RegularResult<Vec<PageInfo>> {
        submit_query!(self.core, list_all_infos_by_chapter_id, step.chapter_id)
    }
}

#[async_trait]
impl<'a> Advance<GetInfoById<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoById<'a>,
    ) -> RegularResult<PageInfo> {
        get_info_by_id(context.conn(), step.id).await
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoExcluded<'a>,
    ) -> RegularResult<PageInfo> {
        get_info_excluded(context.conn(), step.id).await
    }
}

#[async_trait]
impl<'a> Advance<ListInfosByChapterId<'a>, RdbContext>
    for RdbRepoTransactional
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ListInfosByChapterId<'a>,
    ) -> RegularResult<Vec<PageInfo>> {
        list_infos_by_chapter_id(
            context.conn(),
            step.chapter_id,
            step.offset,
            step.limit,
        )
        .await
    }
}

#[async_trait]
impl<'a> Advance<ListAllInfosByChapterId<'a>, RdbContext>
    for RdbRepoTransactional
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ListAllInfosByChapterId<'a>,
    ) -> RegularResult<Vec<PageInfo>> {
        list_all_infos_by_chapter_id(context.conn(), step.chapter_id).await
    }
}

#[async_trait]
impl<'a> Advance<CreateBatch<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &CreateBatch<'a>,
    ) -> RegularResult<Vec<PageInfo>> {
        create_batch(context.conn(), step.forms).await
    }
}

#[async_trait]
impl<'a> Advance<ReserveImage<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ReserveImage<'a>,
    ) -> RegularResult<PageImageReservation> {
        reserve_image(context.conn(), step.id, step.file_ext).await
    }
}

#[async_trait]
impl<'a> Advance<MarkImageUploaded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &MarkImageUploaded<'a>,
    ) -> RegularResult<()> {
        mark_image_uploaded(context.conn(), step.id, step.image_version).await
    }
}

#[async_trait]
impl<'a> Advance<SetUnitCounters<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &SetUnitCounters<'a>,
    ) -> RegularResult<()> {
        set_unit_counters(context.conn(), step.id, step.counters).await
    }
}

#[async_trait]
impl<'a> Advance<DeleteByChapterId<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &DeleteByChapterId<'a>,
    ) -> RegularResult<()> {
        delete_by_chapter_id(context.conn(), step.chapter_id).await
    }
}
#[cfg(all(test, feature = "repo"))]
mod tests;
