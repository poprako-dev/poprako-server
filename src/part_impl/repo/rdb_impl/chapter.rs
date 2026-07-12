//! RDB-backed chapter repository.

use std::collections::HashMap;

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::model::{chapter_model, unit_model};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::step::chapter::{
    AdjustUnitCounters, Create, Delete, FindPinnedInfoByComicId, GetInfoById,
    GetInfoByIdExcluded, ListAllInfosByComicIdExcluded, ListInfos,
    ListPinnedInfosByComicIds, SetPageCounters, UnpinOthers, UpdateInfo,
    UpdateStage,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::rdb_impl::entity::chapter::{
    ChapterAspect, ChapterEntry, ChapterRow,
};
use crate::part_impl::repo::rdb_impl::{RdbRepo, RdbRepoTransactional, incl};
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{RegularError, RegularResult};
use crate::value::chapter::ChapterInclOpt;

use crate::part_impl::repo::rdb_impl::schema::t_chapter::dsl::*;

impl ChapterRepo<RdbContext> for RdbRepo {}

impl ChapterRepoTransactional<RdbContext> for RdbRepoTransactional {}

/// Converts a single `ChapterRow` into a `ChapterInfo`.
fn row_into_info(row: ChapterRow) -> RegularResult<chapter_model::Info> {
    row.try_into()
}

/// Converts a vector of `ChapterRow` values into `ChapterInfo`.
fn rows_into_infos(
    rows: Vec<ChapterRow>,
) -> RegularResult<Vec<chapter_model::Info>> {
    rows.into_iter().map(row_into_info).collect()
}

/// Queries a single chapter row by ID and populates its includes.
async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[ChapterInclOpt],
) -> RegularResult<chapter_model::Info> {
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

/// Queries a single chapter row by ID under `FOR UPDATE` lock.
async fn get_info_by_id_excluded(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[ChapterInclOpt],
) -> RegularResult<chapter_model::Info> {
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

/// Queries chapter rows for a given comic, ordered by index descending.
async fn list_infos(
    conn: &mut RdbConn,
    spec: &chapter_model::ListSpec,
) -> RegularResult<Vec<chapter_model::Info>> {
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
//     offset: u32,
//     limit: u32,
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
//     offset: u32,
//     limit: u32,
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

/// Queries all chapter rows for a comic under `FOR UPDATE` lock.
async fn list_all_infos_by_comic_id_excluded(
    conn: &mut RdbConn,
    comic_id: &str,
) -> RegularResult<Vec<chapter_model::Info>> {
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

/// Finds the pinned chapter for a given comic ID, if one exists.
async fn find_pinned_info_by_comic_id(
    conn: &mut RdbConn,
    comic_id: &str,
    incl_opt: &[ChapterInclOpt],
) -> RegularResult<Option<chapter_model::Info>> {
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

/// Returns a map of comic ID to pinned chapter info for the given comic IDs.
async fn list_pinned_infos_by_comic_ids(
    conn: &mut RdbConn,
    comic_ids: &[String],
) -> RegularResult<HashMap<String, chapter_model::Info>> {
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

/// Inserts a new chapter row from the given form and returns the created info.
async fn create(
    conn: &mut RdbConn,
    form: &chapter_model::Form,
) -> RegularResult<chapter_model::Info> {
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

/// Updates the modifiable fields of a chapter row.
async fn update_info(
    conn: &mut RdbConn,
    update: &chapter_model::InfoUpdate,
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

/// Updates the stage timestamps of a chapter row.
async fn update_stage(
    conn: &mut RdbConn,
    update: &chapter_model::StageUpdate,
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

/// Sets the page and unit counters on a chapter row.
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

/// Adjusts a chapter's unit counters by the given delta.
async fn adjust_unit_counters(
    conn: &mut RdbConn,
    id: &str,
    delta: &unit_model::CounterDelta,
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

/// Unpins all chapters for a comic except the one with the given excluded ID.
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

/// Deletes a single chapter row by ID.
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
    ) -> RegularResult<chapter_model::Info> {
        submit_query!(self.core, get_info_by_id, step.id, step.incl_opt)
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> RegularResult<Vec<chapter_model::Info>> {
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
    ) -> RegularResult<Option<chapter_model::Info>> {
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
    ) -> RegularResult<HashMap<String, chapter_model::Info>> {
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
    ) -> RegularResult<chapter_model::Info> {
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
    ) -> RegularResult<chapter_model::Info> {
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
    ) -> RegularResult<chapter_model::Info> {
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
    ) -> RegularResult<Vec<chapter_model::Info>> {
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
    ) -> RegularResult<Option<chapter_model::Info>> {
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
