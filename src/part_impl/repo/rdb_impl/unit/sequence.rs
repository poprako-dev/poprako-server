//! RDB-backed Unit sequence reads and chain validation.

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;

use crate::model::read::proj::unit::{UnitInfo, UnitOrder};
use crate::part_impl::repo::rdb_impl::entity::unit::UnitInfoRow;
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{
    f_hidden_at, f_id as unit_id, f_next_id as unit_next_id, f_page_id, t_unit,
};
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::result::diesel;

// Number of rows loaded per locked Unit-order query.
const UNIT_ORDER_QUERY_CHUNK_SIZE: i64 = 512;

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
    let mut rows = Vec::new();

    let mut after_id = None::<String>;

    loop {
        //
        let chunk =
            match after_id.as_deref() {
                //
                Some(after_id) => t_unit
                    .filter(f_page_id.eq(page_id))
                    .filter(unit_id.gt(after_id))
                    .select((unit_id, unit_next_id, f_hidden_at))
                    .order(unit_id.asc())
                    .limit(UNIT_ORDER_QUERY_CHUNK_SIZE)
                    .for_update()
                    .load::<(String, Option<String>, Option<OffsetDateTime>)>(
                        conn,
                    )
                    .await,

                None => t_unit
                    .filter(f_page_id.eq(page_id))
                    .select((unit_id, unit_next_id, f_hidden_at))
                    .order(unit_id.asc())
                    .limit(UNIT_ORDER_QUERY_CHUNK_SIZE)
                    .for_update()
                    .load::<(String, Option<String>, Option<OffsetDateTime>)>(
                        conn,
                    )
                    .await,
            }
            .map_err(diesel)?;

        let chunk_is_full = i64::try_from(chunk.len())
            .is_ok_and(|chunk_len| chunk_len == UNIT_ORDER_QUERY_CHUNK_SIZE);

        after_id = chunk.last().map(|(id, _, _)| id.clone());

        rows.extend(chunk);

        if !chunk_is_full {
            break;
        }
    }

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
