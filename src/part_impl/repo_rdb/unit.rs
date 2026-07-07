//! RDB-backed unit repository.

use async_trait::async_trait;
use diesel::dsl::max;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::model::unit::{
    UnitCounters, UnitIndex, UnitIndexUpdate, UnitInfo, UnitOper, UnitPayload,
};
use crate::part::repo::step::unit::{
    CountByPageId, DeleteByIdInPage, ListAllInfosByPageId, ListIndexesByPageId, ListInfosByPageId,
    SaveInfo, UpdateIndexesByPageId,
};
use crate::part::repo::unit::{UnitRepo, UnitRepoTransactional};
use crate::part::shared::execute::Execute;
use crate::part_impl::rdb_core::RdbConn;
use crate::part_impl::rdb_core::RdbContext;
use crate::part_impl::rdb_core::result::{diesel, expected};
use crate::part_impl::repo_rdb::entity::unit::{UnitAspect, UnitEntry, UnitRow};
use crate::part_impl::repo_rdb::{RdbRepo, RdbRepoTransactional};
use crate::result::{RegularError, RegularResult};

use poprako_util::page::Page;

use crate::part_impl::repo_rdb::schema::t_unit::dsl::*;

impl UnitRepo<RdbContext> for RdbRepo {}

impl UnitRepoTransactional<RdbContext> for RdbRepoTransactional {}

async fn list_infos_by_page_id(
    conn: &mut RdbConn,
    page_id: &str,
    page: Page,
) -> RegularResult<Vec<UnitInfo>> {
    let rows: Vec<UnitRow> = t_unit
        .filter(f_page_id.eq(page_id))
        .select(UnitRow::as_select())
        .order_by((f_index.asc(), f_id.asc()))
        .limit(page.limit as i64)
        .offset(page.offset as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

async fn list_all_infos_by_page_id(
    conn: &mut RdbConn,
    page_id: &str,
) -> RegularResult<Vec<UnitInfo>> {
    let rows: Vec<UnitRow> = t_unit
        .filter(f_page_id.eq(page_id))
        .select(UnitRow::as_select())
        .order_by((f_index.asc(), f_id.asc()))
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

async fn next_index(conn: &mut RdbConn, page_id: &str) -> RegularResult<i32> {
    let current: Option<i32> = t_unit
        .filter(f_page_id.eq(page_id))
        .select(max(f_index))
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(current.map(|index| index + 1).unwrap_or(0))
}

async fn create_unit(
    conn: &mut RdbConn,
    page_id: &str,
    id: &str,
    payload: &UnitPayload,
) -> RegularResult<()> {
    let index = next_index(conn, page_id).await?;

    let entry = UnitEntry::new(id, page_id, index, payload);

    diesel::insert_into(t_unit)
        .values(&entry)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn save_unit(
    conn: &mut RdbConn,
    page_id: &str,
    id: &str,
    payload: &UnitPayload,
) -> RegularResult<()> {
    let existing_page_id: Option<String> = t_unit
        .filter(f_id.eq(id))
        .select(f_page_id)
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(existing_page_id) = existing_page_id else {
        return create_unit(conn, page_id, id, payload).await;
    };

    if existing_page_id != page_id {
        return Err(expected("error-unit-duplicate"));
    }

    let now = OffsetDateTime::now_utc();

    let aspect = UnitAspect::new(now).payload(payload);

    diesel::update(t_unit.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn save_info(conn: &mut RdbConn, page_id: &str, oper: &UnitOper) -> RegularResult<()> {
    let (id, payload) = match oper {
        UnitOper::Save {
            id: Some(id),
            payload,
            ..
        } => (id, payload),
        _ => return Err(expected("error-invalid-unit-oper")),
    };

    save_unit(conn, page_id, id, payload).await
}

async fn delete_by_id_in_page(conn: &mut RdbConn, page_id: &str, id: &str) -> RegularResult<()> {
    diesel::delete(t_unit.filter(f_page_id.eq(page_id)).filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn list_indexes_by_page_id(
    conn: &mut RdbConn,
    page_id: &str,
) -> RegularResult<Vec<UnitIndex>> {
    let indexes: Vec<(String, i32)> = t_unit
        .filter(f_page_id.eq(page_id))
        .select((f_id, f_index))
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(indexes
        .into_iter()
        .map(|(id, index)| UnitIndex { id, index })
        .collect())
}

async fn update_indexes_by_page_id(
    conn: &mut RdbConn,
    page_id: &str,
    updates: &[UnitIndexUpdate],
) -> RegularResult<()> {
    for unit_index_update in updates {
        let now = OffsetDateTime::now_utc();

        let aspect = UnitAspect::new(now).index(unit_index_update.index);

        let affected = diesel::update(
            t_unit
                .filter(f_page_id.eq(page_id))
                .filter(f_id.eq(unit_index_update.id.as_str())),
        )
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

        if affected == 0 {
            return Err(expected("error-unit-not-found"));
        }
    }
    Ok(())
}

async fn count_by_page_id(conn: &mut RdbConn, page_id: &str) -> RegularResult<UnitCounters> {
    let infos = list_all_infos_by_page_id(conn, page_id).await?;

    let counters = infos
        .iter()
        .fold(UnitCounters::default(), |mut counters, unit_info| {
            counters.total_unit_count += 1;

            if unit_info.is_translated() {
                counters.translated_unit_count += 1;
            }

            if unit_info.is_proofread {
                counters.proofread_unit_count += 1;
            }

            counters
        });

    Ok(counters)
}

#[async_trait]
impl<'a> Execute<ListInfosByPageId<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &ListInfosByPageId<'a>) -> RegularResult<Vec<UnitInfo>> {
        submit_query!(self.core, list_infos_by_page_id, step.page_id, step.page)
    }
}

#[async_trait]
impl<'a> Execute<ListAllInfosByPageId<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &ListAllInfosByPageId<'a>) -> RegularResult<Vec<UnitInfo>> {
        submit_query!(self.core, list_all_infos_by_page_id, step.page_id)
    }
}

#[async_trait]
impl<'a> Advance<ListInfosByPageId<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ListInfosByPageId<'a>,
    ) -> RegularResult<Vec<UnitInfo>> {
        list_infos_by_page_id(context.conn(), step.page_id, step.page).await
    }
}

#[async_trait]
impl<'a> Advance<ListAllInfosByPageId<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ListAllInfosByPageId<'a>,
    ) -> RegularResult<Vec<UnitInfo>> {
        list_all_infos_by_page_id(context.conn(), step.page_id).await
    }
}

#[async_trait]
impl<'a> Advance<SaveInfo<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(&self, context: &mut RdbContext, step: &SaveInfo<'a>) -> RegularResult<()> {
        save_info(context.conn(), step.page_id, step.oper).await
    }
}

#[async_trait]
impl<'a> Advance<DeleteByIdInPage<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &DeleteByIdInPage<'a>,
    ) -> RegularResult<()> {
        delete_by_id_in_page(context.conn(), step.page_id, step.id).await
    }
}

#[async_trait]
impl<'a> Advance<ListIndexesByPageId<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ListIndexesByPageId<'a>,
    ) -> RegularResult<Vec<UnitIndex>> {
        list_indexes_by_page_id(context.conn(), step.page_id).await
    }
}

#[async_trait]
impl<'a> Advance<UpdateIndexesByPageId<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &UpdateIndexesByPageId<'a>,
    ) -> RegularResult<()> {
        update_indexes_by_page_id(context.conn(), step.page_id, step.updates).await
    }
}

#[async_trait]
impl<'a> Advance<CountByPageId<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &CountByPageId<'a>,
    ) -> RegularResult<UnitCounters> {
        count_by_page_id(context.conn(), step.page_id).await
    }
}
#[cfg(all(test, feature = "repo"))]
mod tests;
