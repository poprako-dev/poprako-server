//! RDB-backed chapter repository.

use std::collections::HashMap;

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::model::chapter::{
    ChapterForm, ChapterInfo, ChapterInfoUpdate, ChapterListSpec,
    ChapterStageUpdate,
};
use crate::model::unit::UnitCounterDelta;
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::step::chapter::{
    AdjustUnitCounters, Create, Delete, FindPinnedInfoByComicId, GetInfoById,
    GetInfoByIdExcluded, ListAllInfosByComicIdExcluded, ListInfos,
    ListPinnedInfosByComicIds, SetPageCounters, UnpinOthers, UpdateInfo,
    UpdateStage,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::rdb_core::RdbConn;
use crate::part_impl::rdb_core::RdbContext;
use crate::part_impl::rdb_core::result::{diesel, expected};
use crate::part_impl::repo_rdb::entity::chapter::{
    ChapterAspect, ChapterEntry, ChapterRow,
};
use crate::part_impl::repo_rdb::incl;
use crate::part_impl::repo_rdb::{RdbRepo, RdbRepoTransactional};
use crate::result::{RegularError, RegularResult};
use crate::value::chapter::ChapterInclOpt;

use crate::part_impl::repo_rdb::schema::t_chapter::dsl::*;

impl ChapterRepo<RdbContext> for RdbRepo {}

impl ChapterRepoTransactional<RdbContext> for RdbRepoTransactional {}

fn row_into_info(row: ChapterRow) -> RegularResult<ChapterInfo> {
    row.try_into()
}

fn rows_into_infos(rows: Vec<ChapterRow>) -> RegularResult<Vec<ChapterInfo>> {
    rows.into_iter().map(row_into_info).collect()
}

async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[ChapterInclOpt],
) -> RegularResult<ChapterInfo> {
    //
    let row: ChapterRow = t_chapter
        .filter(f_id.eq(id))
        .select(ChapterRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-chapter-not-found"))?;

    let mut info = row_into_info(row)?;

    incl::chapter::populate_chapter_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    Ok(info)
}

async fn get_info_by_id_excluded(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[ChapterInclOpt],
) -> RegularResult<ChapterInfo> {
    //
    let row: ChapterRow = t_chapter
        .filter(f_id.eq(id))
        .select(ChapterRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-chapter-not-found"))?;

    let mut info = row_into_info(row)?;

    incl::chapter::populate_chapter_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    Ok(info)
}

async fn list_infos(
    conn: &mut RdbConn,
    spec: &ChapterListSpec,
) -> RegularResult<Vec<ChapterInfo>> {
    //
    let rows: Vec<ChapterRow> = t_chapter
        .filter(f_comic_id.eq(spec.comic_id.as_str()))
        .select(ChapterRow::as_select())
        .order_by(f_index.desc())
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    let mut infos = rows_into_infos(rows)?;

    incl::chapter::populate_chapter_incls(conn, &mut infos, &spec.incl_opt)
        .await?;

    Ok(infos)
}

// async fn list_infos_by_comic_id(
//     conn: &mut RdbConn,
//     comic_id: &str,
//     offset: u64,
//     limit: u64,
// ) -> RegularResult<Vec<ChapterInfo>> {
//     let rows: Vec<ChapterRow> = t_chapter
//         .filter(f_comic_id.eq(comic_id))
//         .select(ChapterRow::as_select())
//         .order_by(f_index.desc())
//         .offset(offset as i64)
//         .limit(limit as i64)
//         .load(conn)
//         .await
//         .map_err(diesel)?;
//
//     rows_into_infos(rows)
// }
//
// async fn list_infos_by_comic_id_excluded(
//     conn: &mut RdbConn,
//     comic_id: &str,
//     offset: u64,
//     limit: u64,
// ) -> RegularResult<Vec<ChapterInfo>> {
//     let rows: Vec<ChapterRow> = t_chapter
//         .filter(f_comic_id.eq(comic_id))
//         .select(ChapterRow::as_select())
//         .order_by(f_index.desc())
//         .offset(offset as i64)
//         .limit(limit as i64)
//         .for_update()
//         .load(conn)
//         .await
//         .map_err(diesel)?;
//
//     rows_into_infos(rows)
// }

async fn list_all_infos_by_comic_id_excluded(
    conn: &mut RdbConn,
    comic_id: &str,
) -> RegularResult<Vec<ChapterInfo>> {
    //
    let rows: Vec<ChapterRow> = t_chapter
        .filter(f_comic_id.eq(comic_id))
        .select(ChapterRow::as_select())
        .order_by(f_index.desc())
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    rows_into_infos(rows)
}

async fn find_pinned_info_by_comic_id(
    conn: &mut RdbConn,
    comic_id: &str,
    incl_opt: &[ChapterInclOpt],
) -> RegularResult<Option<ChapterInfo>> {
    //
    let row: Option<ChapterRow> = t_chapter
        .filter(f_comic_id.eq(comic_id))
        .filter(f_is_pinned.eq(true))
        .select(ChapterRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
        return Ok(None);
    };

    let mut info = row_into_info(row)?;

    incl::chapter::populate_chapter_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    Ok(Some(info))
}

async fn list_pinned_infos_by_comic_ids(
    conn: &mut RdbConn,
    comic_ids: &[String],
) -> RegularResult<HashMap<String, ChapterInfo>> {
    //
    if comic_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows: Vec<ChapterRow> = t_chapter
        .filter(f_comic_id.eq_any(comic_ids))
        .filter(f_is_pinned.eq(true))
        .select(ChapterRow::as_select())
        .load(conn)
        .await
        .map_err(diesel)?;

    let mut map = HashMap::with_capacity(rows.len());

    for row in rows {
        //
        let info = row_into_info(row)?;

        map.insert(info.comic_id.clone(), info);
    }

    Ok(map)
}

async fn create(
    conn: &mut RdbConn,
    form: &ChapterForm,
) -> RegularResult<ChapterInfo> {
    //
    let entry = ChapterEntry::from(form);

    let row: ChapterRow = diesel::insert_into(t_chapter)
        .values(&entry)
        .returning(ChapterRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    row_into_info(row)
}

async fn update_info(
    conn: &mut RdbConn,
    update: &ChapterInfoUpdate,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let mut aspect = ChapterAspect::new(now);

    if let Some(subtitle) = &update.subtitle {
        aspect = aspect.subtitle(subtitle);
    }

    if let Some(pin) = update.pin {
        aspect = aspect.pinned(pin);
    }

    diesel::update(t_chapter.filter(f_id.eq(update.id.as_str())))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn update_stage(
    conn: &mut RdbConn,
    update: &ChapterStageUpdate,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = ChapterAspect::new(now).stages(update.stages, now);

    diesel::update(t_chapter.filter(f_id.eq(update.id.as_str())))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn set_page_counters(
    conn: &mut RdbConn,
    id: &str,
    page_count: i32,
    total_unit_count: i32,
    translated_unit_count: i32,
    proofread_unit_count: i32,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = ChapterAspect::new(now)
        .page_count(page_count)
        .total_unit_count(total_unit_count)
        .translated_unit_count(translated_unit_count)
        .proofread_unit_count(proofread_unit_count);

    diesel::update(t_chapter.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn adjust_unit_counters(
    conn: &mut RdbConn,
    id: &str,
    delta: &UnitCounterDelta,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    diesel::update(t_chapter.filter(f_id.eq(id)))
        .set((
            f_total_unit_count.eq(f_total_unit_count + delta.total_unit_count),
            f_translated_unit_count
                .eq(f_translated_unit_count + delta.translated_unit_count),
            f_proofread_unit_count
                .eq(f_proofread_unit_count + delta.proofread_unit_count),
            f_updated_at.eq(now),
        ))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn unpin_others(
    conn: &mut RdbConn,
    comic_id: &str,
    excluded_id: &str,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    diesel::update(
        t_chapter
            .filter(f_comic_id.eq(comic_id))
            .filter(f_id.ne(excluded_id)),
    )
    .set((f_is_pinned.eq(false), f_updated_at.eq(now)))
    .execute(conn)
    .await
    .map_err(diesel)?;

    Ok(())
}

async fn delete(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    //
    diesel::delete(t_chapter.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &GetInfoById<'a>,
    ) -> RegularResult<ChapterInfo> {
        submit_query!(self.core, get_info_by_id, step.id, step.incl_opt)
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> RegularResult<Vec<ChapterInfo>> {
        submit_query!(self.core, list_infos, step.spec)
    }
}

// #[async_trait]
// impl<'a> Execute<ListInfosByComicId<'a>> for RdbRepo {
//     ...
// }

#[async_trait]
impl<'a> Execute<FindPinnedInfoByComicId<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &FindPinnedInfoByComicId<'a>,
    ) -> RegularResult<Option<ChapterInfo>> {
        submit_query!(
            self.core,
            find_pinned_info_by_comic_id,
            step.comic_id,
            step.incl_opt
        )
    }
}

#[async_trait]
impl<'a> Execute<ListPinnedInfosByComicIds<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListPinnedInfosByComicIds<'a>,
    ) -> RegularResult<HashMap<String, ChapterInfo>> {
        submit_query!(self.core, list_pinned_infos_by_comic_ids, step.comic_ids)
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Create<'a>,
    ) -> RegularResult<ChapterInfo> {
        create(context.conn(), step.form).await
    }
}

#[async_trait]
impl<'a> Advance<GetInfoById<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoById<'a>,
    ) -> RegularResult<ChapterInfo> {
        get_info_by_id(context.conn(), step.id, step.incl_opt).await
    }
}

#[async_trait]
impl<'a> Advance<GetInfoByIdExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoByIdExcluded<'a>,
    ) -> RegularResult<ChapterInfo> {
        get_info_by_id_excluded(context.conn(), step.id, step.incl_opt).await
    }
}

// #[async_trait]
// impl<'a> Advance<ListInfosByComicIdExcluded<'a>, RdbContext> for RdbRepoTransactional {
//     ...
// }

#[async_trait]
impl<'a> Advance<ListAllInfosByComicIdExcluded<'a>, RdbContext>
    for RdbRepoTransactional
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ListAllInfosByComicIdExcluded<'a>,
    ) -> RegularResult<Vec<ChapterInfo>> {
        list_all_infos_by_comic_id_excluded(context.conn(), step.comic_id).await
    }
}

#[async_trait]
impl<'a> Advance<FindPinnedInfoByComicId<'a>, RdbContext>
    for RdbRepoTransactional
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &FindPinnedInfoByComicId<'a>,
    ) -> RegularResult<Option<ChapterInfo>> {
        find_pinned_info_by_comic_id(
            context.conn(),
            step.comic_id,
            step.incl_opt,
        )
        .await
    }
}

#[async_trait]
impl<'a> Advance<UpdateInfo<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &UpdateInfo<'a>,
    ) -> RegularResult<()> {
        update_info(context.conn(), step.update).await
    }
}

#[async_trait]
impl<'a> Advance<UpdateStage<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &UpdateStage<'a>,
    ) -> RegularResult<()> {
        update_stage(context.conn(), step.update).await
    }
}

#[async_trait]
impl<'a> Advance<SetPageCounters<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &SetPageCounters<'a>,
    ) -> RegularResult<()> {
        set_page_counters(
            context.conn(),
            step.id,
            step.page_count,
            step.total_unit_count,
            step.translated_unit_count,
            step.proofread_unit_count,
        )
        .await
    }
}

#[async_trait]
impl<'a> Advance<AdjustUnitCounters<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &AdjustUnitCounters<'a>,
    ) -> RegularResult<()> {
        adjust_unit_counters(context.conn(), step.id, &step.delta).await
    }
}

#[async_trait]
impl<'a> Advance<UnpinOthers<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &UnpinOthers<'a>,
    ) -> RegularResult<()> {
        unpin_others(context.conn(), step.comic_id, step.excluded_id).await
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Delete<'a>,
    ) -> RegularResult<()> {
        delete(context.conn(), step.id).await
    }
}
#[cfg(all(test, feature = "repo"))]
mod tests;
