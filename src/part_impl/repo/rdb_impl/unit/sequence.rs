//! RDB-backed Unit sequence reads and chain validation.

#[cfg(test)]
mod tests;

use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use crate::model::read::proj::unit::{UnitInfo, UnitOrder};
use crate::part_impl::repo::rdb_impl::entity::unit::UnitInfoRow;
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{
    f_hidden_at, f_id, f_next_id, f_page_id, t_unit,
};
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::RdbConn;
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

    for index in 0..units.len() {
        //
        if units[index + 1..]
            .iter()
            .any(|unit| id_of(unit) == id_of(&units[index]))
        {
            return Err(corrupt_unit_chain_err());
        }
    }

    let mut head_pos = None;

    for cand in 0..units.len() {
        //
        let has_predecessor = units.iter().any(|unit| {
            //
            next_id_of(unit)
                .is_some_and(|next_id| next_id == id_of(&units[cand]))
        });

        if has_predecessor {
            continue;
        }

        if head_pos.replace(cand).is_some() {
            return Err(corrupt_unit_chain_err());
        }
    }

    let Some(head_pos) = head_pos else {
        return Err(corrupt_unit_chain_err());
    };

    units.swap(0, head_pos);

    for index in 0..units.len() - 1 {
        //
        let next_pos =
            units[index + 1..]
                .iter()
                .enumerate()
                .find_map(|(pos, cand)| {
                    //
                    next_id_of(&units[index])
                        .is_some_and(|next_id| next_id == id_of(cand))
                        .then_some(pos)
                });

        let Some(next_pos) = next_pos else {
            return Err(corrupt_unit_chain_err());
        };

        units.swap(index + 1, index + 1 + next_pos);
    }

    if units.last().is_some_and(|unit| next_id_of(unit).is_some()) {
        return Err(corrupt_unit_chain_err());
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

#[instrument(level = "info", skip_all)]
/// Lists every requested persisted Unit that currently exists.
pub async fn list_infos_by_ids(
    conn: &mut RdbConn,
    ids: &[String],
) -> BaseRest<Vec<UnitInfo>> {
    //
    let rows = t_unit
        .filter(f_id.eq_any(ids))
        .select(UnitInfoRow::as_select())
        .load::<UnitInfoRow>(conn)
        .await
        .map_err(diesel)?;

    accept(rows.into_iter().map(UnitInfo::from).collect())
}

#[instrument(level = "info", skip_all)]
/// Locks and lists the complete Unit chain, including tombstones.
pub async fn list_orders_for_update(
    conn: &mut RdbConn,
    page_id: &str,
) -> BaseRest<Vec<UnitOrder>> {
    //
    let rows = t_unit
        .filter(f_page_id.eq(page_id))
        .select((f_id, f_next_id, f_hidden_at))
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
