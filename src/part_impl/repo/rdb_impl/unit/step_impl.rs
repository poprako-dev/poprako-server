//! RDB-backed Unit operations.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use tracing::instrument;

use crate::model::read::proj::unit::{UnitCounters, UnitInfo, UnitOrder};
use crate::model::write::unit::UnitEdit;
use crate::part_impl::repo::rdb_impl::entity::unit::{
    UnitAspect, UnitEntry, UnitRow,
};
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::*;
use crate::part_impl::shared::RdbConn;
use crate::part_impl::shared::result::{diesel, expected};
use crate::result::{BaseError, BaseResult, accept};

#[cfg(test)]
mod tests;

fn order_units<T, I, N>(
    units: &mut [T],
    id_of: I,
    next_id_of: N,
) -> BaseResult<()>
where
    I: for<'a> Fn(&'a T) -> &'a str,
    N: for<'a> Fn(&'a T) -> Option<&'a str>,
{
    //
    if units.is_empty() {
        return accept(());
    }

    for index in 0..units.len() {
        if units[index + 1..]
            .iter()
            .any(|unit| id_of(unit) == id_of(&units[index]))
        {
            return Err(corrupt_unit_chain_err());
        }
    }

    let mut head_position = None;

    for candidate in 0..units.len() {
        //
        let has_predecessor = units.iter().any(|unit| {
            next_id_of(unit)
                .is_some_and(|next_id| next_id == id_of(&units[candidate]))
        });

        if has_predecessor {
            continue;
        }

        if head_position.replace(candidate).is_some() {
            return Err(corrupt_unit_chain_err());
        }
    }

    let Some(head_position) = head_position else {
        return Err(corrupt_unit_chain_err());
    };

    units.swap(0, head_position);

    for index in 0..units.len() - 1 {
        //
        let next_position = units[index + 1..].iter().position(|candidate| {
            next_id_of(&units[index])
                .is_some_and(|next_id| next_id == id_of(candidate))
        });

        let Some(next_position) = next_position else {
            return Err(corrupt_unit_chain_err());
        };

        units.swap(index + 1, index + 1 + next_position);
    }

    if units.last().is_some_and(|unit| next_id_of(unit).is_some()) {
        return Err(corrupt_unit_chain_err());
    }

    accept(())
}

fn corrupt_unit_chain_err() -> BaseError {
    BaseError::Unrecoverable {
        message: "persisted Unit chain is corrupt".to_string(),
    }
}

#[instrument(level = "info", err(Debug), skip_all)]
/// Lists visible Units in verified linked-list order.
pub async fn list_infos(
    conn: &mut RdbConn,
    page_id: &str,
) -> BaseResult<Vec<UnitInfo>> {
    //
    let rows = t_unit
        .filter(f_page_id.eq(page_id))
        .select(UnitRow::as_select())
        .load::<UnitRow>(conn)
        .await
        .map_err(diesel)?;

    let mut unit_infos =
        rows.into_iter().map(UnitInfo::from).collect::<Vec<_>>();

    order_units(
        &mut unit_infos,
        |unit_info| unit_info.id.as_str(),
        |unit_info| unit_info.next_id.as_deref(),
    )?;

    unit_infos.retain(|unit_info| unit_info.hidden_at.is_none());

    accept(unit_infos)
}

#[instrument(level = "info", err(Debug), skip_all)]
/// Locks and lists the complete Unit chain, including tombstones.
pub async fn list_positions_for_update(
    conn: &mut RdbConn,
    page_id: &str,
) -> BaseResult<Vec<UnitOrder>> {
    //
    let positions = t_unit
        .filter(f_page_id.eq(page_id))
        .select((f_id, f_next_id, f_hidden_at))
        .for_update()
        .load::<(String, Option<String>, Option<OffsetDateTime>)>(conn)
        .await
        .map_err(diesel)?;

    let mut unit_orders = positions
        .into_iter()
        .map(|(id, next_id, hidden_at)| UnitOrder {
            id,
            next_id,
            is_hidden: hidden_at.is_some(),
        })
        .collect::<Vec<_>>();

    order_units(
        &mut unit_orders,
        |unit_order| unit_order.id.as_str(),
        |unit_order| unit_order.next_id.as_deref(),
    )?;

    accept(unit_orders)
}

#[instrument(level = "info", err(Debug), skip_all)]
/// Applies normalized Unit edits and returns the latest visible counters.
pub async fn apply_edits(
    conn: &mut RdbConn,
    page_id: &str,
    orders: &[UnitOrder],
    edits: &[UnitEdit],
) -> BaseResult<UnitCounters> {
    //
    for edit in edits {
        match edit {
            //
            UnitEdit::Delete { id } => {
                //
                let affected = diesel::update(
                    t_unit.filter(f_page_id.eq(page_id)).filter(f_id.eq(id)),
                )
                .set(UnitAspect::new().hide())
                .execute(conn)
                .await
                .map_err(diesel)?;

                if affected != 1 {
                    return Err(expected("error-invalid-unit-oper"));
                }
            }

            UnitEdit::Save { id, .. } => {
                //
                let affected = diesel::update(
                    t_unit.filter(f_page_id.eq(page_id)).filter(f_id.eq(id)),
                )
                .set(UnitAspect::new().apply_edit(edit))
                .execute(conn)
                .await
                .map_err(diesel)?;

                if affected == 1 {
                    continue;
                }

                let Some(entry) = UnitEntry::from_edit(page_id, None, edit)
                else {
                    return Err(expected("error-invalid-unit-oper"));
                };

                diesel::insert_into(t_unit)
                    .values(entry)
                    .execute(conn)
                    .await
                    .map_err(diesel)?;
            }
        }
    }

    for order in orders {
        //
        let affected = diesel::update(
            t_unit
                .filter(f_page_id.eq(page_id))
                .filter(f_id.eq(order.id.as_str())),
        )
        .set(UnitAspect::new().order(order.next_id.as_deref()))
        .execute(conn)
        .await
        .map_err(diesel)?;

        if affected != 1 {
            return Err(expected("error-invalid-unit-oper"));
        }
    }

    let unit_infos = list_infos(conn, page_id).await?;

    accept(count_infos(&unit_infos))
}

fn count_infos(unit_infos: &[UnitInfo]) -> UnitCounters {
    unit_infos.iter().fold(
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
    )
}
