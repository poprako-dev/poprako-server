//! RDB-backed Unit sequence reads and chain validation.

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use diesel::PgExpressionMethods as _;
use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel::sql_types::Bool;
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;

use crate::model::read::proj::unit::{UnitInfo, UnitOrder};
use crate::part_impl::repo::rdb_impl::entity::unit::UnitInfoRow;
use crate::part_impl::repo::rdb_impl::schema::t_page::dsl::{
    f_chapter_id as page_chapter_id, f_id as page_row_id,
    f_index as page_index, t_page,
};
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{
    f_hidden_at, f_id as unit_id, f_next_id as unit_next_id, f_page_id,
    f_proofread_text, f_translated_text, t_unit,
};
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::result::diesel;

/// Returns an unrecoverable error for a corrupt Unit chain.
pub fn corrupt_unit_chain_err() -> BaseError {
    //
    BaseError::Unrecoverable {
        message: "persisted Unit chain is corrupt".to_string(),
    }
}

/// Orders Units in linked-list order, detecting cycles and multiple heads.
pub fn order_units<T, I, N>(
    units: &mut [T],
    id_of: I,
    next_id_of: N,
) -> BaseRest<()>
where
    I: for<'a> Fn(&'a T) -> &'a str,
    N: for<'a> Fn(&'a T) -> Option<&'a str>,
{
    //
    if units.is_empty() {
        return accept(());
    }

    let mut index_by_id = HashMap::with_capacity(units.len());

    for (index, unit) in units.iter().enumerate() {
        //
        if index_by_id.insert(id_of(unit), index).is_some() {
            return Err(corrupt_unit_chain_err());
        }
    }

    let mut next_index_by_index = Vec::with_capacity(units.len());

    let mut has_predecessor = vec![false; units.len()];

    for unit in units.iter() {
        //
        let next_index = match next_id_of(unit) {
            //
            Some(next_id) => {
                //
                let Some(next_index) = index_by_id.get(next_id).copied() else {
                    return Err(corrupt_unit_chain_err());
                };

                let Some(has_predecessor) = has_predecessor.get_mut(next_index)
                else {
                    return Err(corrupt_unit_chain_err());
                };

                if std::mem::replace(has_predecessor, true) {
                    return Err(corrupt_unit_chain_err());
                }

                Some(next_index)
            }

            None => None,
        };

        next_index_by_index.push(next_index);
    }

    let mut head_indexes = has_predecessor.iter().enumerate().filter_map(
        |(index, has_predecessor)| (!has_predecessor).then_some(index),
    );

    let Some(head_index) = head_indexes.next() else {
        return Err(corrupt_unit_chain_err());
    };

    if head_indexes.next().is_some() {
        return Err(corrupt_unit_chain_err());
    }

    let mut ordered_indexes = Vec::with_capacity(units.len());

    let mut current_index = Some(head_index);

    while let Some(index) = current_index {
        //
        if ordered_indexes.len() >= units.len() {
            return Err(corrupt_unit_chain_err());
        }

        ordered_indexes.push(index);

        let Some(next_index) = next_index_by_index.get(index).copied() else {
            return Err(corrupt_unit_chain_err());
        };

        current_index = next_index;
    }

    if ordered_indexes.len() != units.len() {
        return Err(corrupt_unit_chain_err());
    }

    drop(index_by_id);

    let mut original_index_by_position = (0..units.len()).collect::<Vec<_>>();

    let mut position_by_original_index = (0..units.len()).collect::<Vec<_>>();

    for (target_position, desired_original_index) in
        ordered_indexes.into_iter().enumerate()
    {
        //
        let Some(current_position) = position_by_original_index
            .get(desired_original_index)
            .copied()
        else {
            return Err(corrupt_unit_chain_err());
        };

        let Some(displaced_original_index) =
            original_index_by_position.get(target_position).copied()
        else {
            return Err(corrupt_unit_chain_err());
        };

        debug_assert!(target_position < units.len());

        debug_assert!(current_position < units.len());

        if target_position >= units.len() || current_position >= units.len() {
            return Err(corrupt_unit_chain_err());
        }

        units.swap(target_position, current_position);

        original_index_by_position.swap(target_position, current_position);

        let Some(desired_position) =
            position_by_original_index.get_mut(desired_original_index)
        else {
            return Err(corrupt_unit_chain_err());
        };

        *desired_position = target_position;

        let Some(displaced_position) =
            position_by_original_index.get_mut(displaced_original_index)
        else {
            return Err(corrupt_unit_chain_err());
        };

        *displaced_position = current_position;
    }

    accept(())
}

/// Lists Units in verified linked-list order for one Page.
pub async fn list_infos(
    conn: &mut RdbConn,
    page_id: &str,
) -> BaseRest<Vec<UnitInfo>> {
    //
    let rows = t_unit
        .filter(f_page_id.eq(page_id))
        .select(UnitInfoRow::as_select())
        .load::<UnitInfoRow>(conn)
        .await
        .map_err(diesel)?;

    let mut unit_infos =
        rows.into_iter().map(UnitInfo::from).collect::<Vec<_>>();

    order_units(
        &mut unit_infos,
        |unit_info| unit_info.id.as_str(),
        |unit_info| unit_info.next_id.as_deref(),
    )?;

    accept(unit_infos)
}

/// Lists Units for multiple Pages with one query and validates each linked list.
pub async fn list_infos_by_page_ids(
    conn: &mut RdbConn,
    page_ids: &[String],
) -> BaseRest<Vec<UnitInfo>> {
    //
    let rows = t_unit
        .filter(f_page_id.eq_any(page_ids))
        .select(UnitInfoRow::as_select())
        .load::<UnitInfoRow>(conn)
        .await
        .map_err(diesel)?;

    let mut unit_infos_by_page_id = HashMap::<String, Vec<UnitInfo>>::new();

    for row in rows {
        //
        let unit_info = UnitInfo::from(row);

        unit_infos_by_page_id
            .entry(unit_info.page_id.clone())
            .or_default()
            .push(unit_info);
    }

    let mut unit_infos = Vec::new();

    for page_id in page_ids {
        //
        let mut page_unit_infos =
            unit_infos_by_page_id.remove(page_id).unwrap_or_default();

        order_units(
            &mut page_unit_infos,
            |unit_info| unit_info.id.as_str(),
            |unit_info| unit_info.next_id.as_deref(),
        )?;

        unit_infos.extend(page_unit_infos);
    }

    accept(unit_infos)
}

/// Lists Chapter Page IDs containing at least one visible text diff.
#[instrument(level = "info", skip_all)]
pub async fn list_editted_diff_page_ids(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<Vec<String>> {
    // Keep the Unit check correlated so PostgreSQL can stop after one match
    // for each Page.
    let has_editted_diff = diesel::dsl::exists(
        t_unit
            .filter(f_page_id.eq(page_row_id))
            .filter(f_hidden_at.is_null())
            .filter(diesel::dsl::sql::<Bool>(
                r#""t_unit"."f_proofread_text" !~ '^\s*$'"#,
            ))
            .filter(f_proofread_text.is_distinct_from(f_translated_text)),
    );

    let page_ids = t_page
        .filter(page_chapter_id.eq(chapter_id))
        .filter(has_editted_diff)
        .select(page_row_id)
        .order(page_index.asc())
        .load::<String>(conn)
        .await
        .map_err(diesel)?;

    accept(page_ids)
}

/// Lists every requested persisted Unit that currently exists.
#[instrument(level = "info", skip_all)]
pub async fn list_infos_by_ids(
    conn: &mut RdbConn,
    ids: &[String],
) -> BaseRest<Vec<UnitInfo>> {
    //
    let rows = t_unit
        .filter(unit_id.eq_any(ids))
        .select(UnitInfoRow::as_select())
        .load::<UnitInfoRow>(conn)
        .await
        .map_err(diesel)?;

    accept(rows.into_iter().map(UnitInfo::from).collect())
}

/// Locks and lists the complete Unit chain, including tombstones.
#[instrument(level = "info", skip_all)]
pub async fn list_orders_for_update(
    conn: &mut RdbConn,
    page_id: &str,
) -> BaseRest<Vec<UnitOrder>> {
    //
    let rows = t_unit
        .filter(f_page_id.eq(page_id))
        .select((unit_id, unit_next_id, f_hidden_at))
        .for_update()
        .load::<(String, Option<String>, Option<OffsetDateTime>)>(conn)
        .await
        .map_err(diesel)?;

    let mut orders = rows
        .into_iter()
        .map(|(id, next_id, hidden_at)| UnitOrder {
            id,
            next_id,
            is_hidden: hidden_at.is_some(),
        })
        .collect::<Vec<_>>();

    order_units(
        &mut orders,
        |order| order.id.as_str(),
        |order| order.next_id.as_deref(),
    )?;

    accept(orders)
}
