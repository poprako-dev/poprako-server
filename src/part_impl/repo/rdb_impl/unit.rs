//! RDB-backed unit repository.

use diesel::dsl::max;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_util::page::Page;

use crate::part::repo::unit::UnitRepo;
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::unit::{
    UnitAspect, UnitEntry, UnitRow,
};
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::RegularResult;

use crate::model::unit::{
    UnitContent, UnitCounters, UnitIndex, UnitIndexUpdate, UnitInfo,
};
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::*;

impl UnitRepo<RdbContext> for RdbRepo {}

mod orchestra;

/// Query a paginated list of unit infos for a page, ordered by index then ID.
async fn list_infos_by_page_id(
    conn: &mut RdbConn,
    page_id: &str,
    page: Page,
) -> RegularResult<Vec<UnitInfo>> {
    //
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

/// Query all unit infos for a page, ordered by index then ID (no pagination).
async fn list_all_infos_by_page_id(
    conn: &mut RdbConn,
    page_id: &str,
) -> RegularResult<Vec<UnitInfo>> {
    //
    let rows: Vec<UnitRow> = t_unit
        .filter(f_page_id.eq(page_id))
        .select(UnitRow::as_select())
        .order_by((f_index.asc(), f_id.asc()))
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Compute the next available unit index for a page.
async fn next_index(conn: &mut RdbConn, page_id: &str) -> RegularResult<i32> {
    //
    let current: Option<i32> = t_unit
        .filter(f_page_id.eq(page_id))
        .select(max(f_index))
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(current.map(|index| index + 1).unwrap_or(0))
}

/// Insert a new unit with the next available index for the given page.
async fn create_unit(
    conn: &mut RdbConn,
    page_id: &str,
    id: &str,
    payload: &UnitContent,
) -> RegularResult<()> {
    //
    let index = next_index(conn, page_id).await?;

    let entry = UnitEntry::new(id, page_id, index, payload);

    diesel::insert_into(t_unit)
        .values(&entry)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Upsert a unit: create if absent, otherwise update its payload.
async fn save_unit(
    conn: &mut RdbConn,
    page_id: &str,
    id: &str,
    payload: &UnitContent,
) -> RegularResult<()> {
    //
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

/// Delete a unit by its ID within the scope of a page.
async fn delete_by_id_in_page(
    conn: &mut RdbConn,
    page_id: &str,
    id: &str,
) -> RegularResult<()> {
    //
    diesel::delete(t_unit.filter(f_page_id.eq(page_id)).filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Query (id, index) pairs for all units in a page.
async fn list_indexes_by_page_id(
    conn: &mut RdbConn,
    page_id: &str,
) -> RegularResult<Vec<UnitIndex>> {
    //
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

/// Reorder units in a page by assigning new indexes, safely handling cyclic
/// dependencies via a two-phase shift-then-set strategy.
async fn update_indexes_by_page_id(
    conn: &mut RdbConn,
    page_id: &str,
    updates: &[UnitIndexUpdate],
) -> RegularResult<()> {
    //
    if updates.is_empty() {
        return Ok(());
    }

    // Index shifts can create a cyclic dependency where no sequential
    // ordering avoids a temporary duplicate (page_id, index).  Two-phase:
    //  1. Bump every affected row to index + OFFSET (safe temporary range
    //     with no overlapping values).
    //  2. Set each row to its target index via sequential UPDATEs (now
    //     conflict-free because all rows are in the non-overlapping range).
    const OFFSET: i32 = 100_000;

    let mut id_filters: Vec<&str> = Vec::with_capacity(updates.len());

    for update in updates {
        id_filters.push(update.id.as_str());
    }

    // Phase 1: shift all affected units up by OFFSET in a single UPDATE.
    diesel::update(
        t_unit
            .filter(f_page_id.eq(page_id))
            .filter(f_id.eq_any(&id_filters)),
    )
    .set(f_index.eq(f_index + OFFSET))
    .execute(conn)
    .await
    .map_err(diesel)?;

    // Phase 2: set each unit to its target index, now conflict-free.
    for unit_index_update in updates {
        //
        let now = OffsetDateTime::now_utc();

        let aspect = UnitAspect::new(now).index(unit_index_update.index);

        diesel::update(
            t_unit
                .filter(f_page_id.eq(page_id))
                .filter(f_id.eq(unit_index_update.id.as_str())),
        )
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;
    }

    Ok(())
}

/// Count total, translated, and proofread units for a page.
async fn count_by_page_id(
    conn: &mut RdbConn,
    page_id: &str,
) -> RegularResult<UnitCounters> {
    //
    let infos = list_all_infos_by_page_id(conn, page_id).await?;

    let counters = infos.iter().fold(
        UnitCounters::default(),
        |mut counters, unit_info| {
            //
            counters.total_unit_count += 1;

            if unit_info.is_translated() {
                counters.translated_unit_count += 1;
            }

            if unit_info.is_proofread {
                counters.proofread_unit_count += 1;
            }

            counters
        },
    );

    Ok(counters)
}

#[cfg(all(test, feature = "repo"))]
mod tests;
