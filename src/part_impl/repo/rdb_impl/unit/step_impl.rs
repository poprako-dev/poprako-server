//! RDB-backed Unit operations.

use std::collections::HashSet;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::model::read::proj::unit::{UnitCounters, UnitInfo, UnitOrder};
use crate::model::write::unit::UnitEdit;
use crate::part_impl::repo::rdb_impl::entity::unit::{
    UnitAspect, UnitEntry, UnitRow,
};
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::*;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::RdbConn;
use crate::shared::result::diesel;
use crate::util::Patch;

#[cfg(test)]
mod tests;

/// Lists visible Units in verified linked-list order.
pub async fn list_infos(
    conn: &mut RdbConn,
    page_id: &str,
) -> BaseRest<Vec<UnitInfo>> {
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

    accept(unit_infos)
}

#[instrument(level = "info", err(Debug), skip_all)]
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

#[instrument(level = "info", err(Debug), skip_all)]
/// Applies normalized Unit edits and returns the latest visible counters.
pub async fn apply_edits(
    conn: &mut RdbConn,
    page_id: &str,
    orders: &[UnitOrder],
    edits: &[UnitEdit],
) -> BaseRest<UnitCounters> {
    //
    let ordered_ids = apply_order_edits(orders, edits)?;

    for edit in edits {
        match edit {
            //
            UnitEdit::Create { id, .. } => {
                //
                let Some(entry) = UnitEntry::from_edit(page_id, edit) else {
                    //
                    let error_message = trl("error-invalid-unit-oper");

                    tracing::warn!(
                        error_variant = ?ExpectedVariant::Args,
                        error_message = %error_message,
                        page_id = %page_id,
                        unit_id = %id,
                        operation = "create",
                        stage = "apply_edits",
                        "expected error: invalid unit operation",
                    );

                    return Err(BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: error_message,
                    });
                };

                diesel::insert_into(t_unit)
                    .values(entry)
                    .execute(conn)
                    .await
                    .map_err(diesel)?;
            }

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
                    //
                    let error_message = trl("error-invalid-unit-oper");

                    tracing::warn!(
                        error_variant = ?ExpectedVariant::Args,
                        error_message = %error_message,
                        page_id = %page_id,
                        unit_id = %id,
                        operation = "delete",
                        affected,
                        stage = "apply_edits",
                        "expected error: invalid unit operation",
                    );

                    return Err(BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: error_message,
                    });
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

                if affected != 1 {
                    //
                    let error_message = trl("error-invalid-unit-oper");

                    tracing::warn!(
                        error_variant = ?ExpectedVariant::Args,
                        error_message = %error_message,
                        page_id = %page_id,
                        unit_id = %id,
                        operation = "save",
                        affected,
                        stage = "apply_edits",
                        "expected error: invalid unit operation",
                    );

                    return Err(BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: error_message,
                    });
                }
            }
        }
    }

    for (index, id) in ordered_ids.iter().enumerate() {
        //
        let next_id = ordered_ids.get(index + 1).copied();

        let unchanged = orders
            .iter()
            .find(|order| order.id == **id)
            .is_some_and(|order| order.next_id.as_deref() == next_id);

        if unchanged {
            continue;
        }

        let affected = diesel::update(
            t_unit.filter(f_page_id.eq(page_id)).filter(f_id.eq(*id)),
        )
        .set(UnitAspect::new().order(next_id))
        .execute(conn)
        .await
        .map_err(diesel)?;

        if affected != 1 {
            //
            let error_message = trl("error-invalid-unit-oper");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                error_message = %error_message,
                page_id = %page_id,
                unit_id = %id,
                index,
                next_unit_id = ?next_id,
                operation = "reorder",
                affected,
                stage = "apply_edits",
                "expected error: invalid unit operation",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: error_message,
            });
        }
    }

    let unit_infos = list_infos(conn, page_id).await?;

    accept(count_infos(&unit_infos))
}

// Orders units in linked-list order, detecting cycles and multiple heads.
fn order_units<T, I, N>(
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
        if units[index + 1..]
            .iter()
            .any(|unit| id_of(unit) == id_of(&units[index]))
        {
            return Err(corrupt_unit_chain_err());
        }
    }

    let mut head_pos = None;

    for candidate in 0..units.len() {
        //
        let has_predecessor = units.iter().any(|unit| {
            next_id_of(unit)
                .is_some_and(|next_id| next_id == id_of(&units[candidate]))
        });

        if has_predecessor {
            continue;
        }

        if head_pos.replace(candidate).is_some() {
            return Err(corrupt_unit_chain_err());
        }
    }

    let Some(head_pos) = head_pos else {
        return Err(corrupt_unit_chain_err());
    };

    units.swap(0, head_pos);

    for index in 0..units.len() - 1 {
        //
        let next_pos = units[index + 1..].iter().enumerate().find_map(
            |(pos, candidate)| {
                next_id_of(&units[index])
                    .is_some_and(|next_id| next_id == id_of(candidate))
                    .then_some(pos)
            },
        );

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

// Applies normalized Unit edits to produce the final ordered id list.
fn apply_order_edits<'a>(
    orders: &'a [UnitOrder],
    edits: &'a [UnitEdit],
) -> BaseRest<Vec<&'a str>> {
    //
    let mut ordered_ids = orders
        .iter()
        .map(|order| order.id.as_str())
        .collect::<Vec<_>>();

    let mut hidden_ids = orders
        .iter()
        .filter(|order| order.is_hidden)
        .map(|order| order.id.as_str())
        .collect::<HashSet<_>>();

    for edit in edits {
        //
        let UnitEdit::Create { id, .. } = edit else {
            continue;
        };

        if find_order_pos(&ordered_ids, id).is_some() {
            //
            let error_message = trl("error-invalid-unit-oper");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                error_message = %error_message,
                unit_id = %id,
                existing_order_count = ordered_ids.len(),
                operation = "create",
                stage = "apply_order_edits",
                "expected error: invalid unit operation",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: error_message,
            });
        }

        ordered_ids.push(id);

        hidden_ids.remove(id.as_str());
    }

    for edit in edits {
        match edit {
            //
            UnitEdit::Create { id, next_id, .. } => {
                move_order(&mut ordered_ids, id, next_id.as_deref())?;
            }

            UnitEdit::Save { id, next_id, .. } => {
                //
                hidden_ids.remove(id.as_str());

                match next_id {
                    //
                    Patch::Skip => {}

                    Patch::Clear => {
                        move_order(&mut ordered_ids, id, None)?;
                    }

                    Patch::Assign(next_id) => {
                        move_order(&mut ordered_ids, id, Some(next_id))?;
                    }
                }
            }

            UnitEdit::Delete { id } => {
                hidden_ids.insert(id);
            }
        }
    }

    let visible_count = ordered_ids
        .iter()
        .filter(|id| !hidden_ids.contains(**id))
        .count();

    if visible_count > 100 {
        //
        let error_message = trl("error-invalid-unit-oper");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            error_message = %error_message,
            visible_count,
            max_visible_count = 100,
            operation = "reorder",
            stage = "apply_order_edits",
            "expected error: invalid unit operation",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: error_message,
        });
    }

    accept(ordered_ids)
}

// Computes visible Unit counters from the ordered unit info list.
fn count_infos(unit_infos: &[UnitInfo]) -> UnitCounters {
    unit_infos
        .iter()
        .filter(|unit_info| unit_info.hidden_at.is_none())
        .fold(UnitCounters::default(), |mut counters, unit_info| {
            //
            counters.total_unit_count += 1;

            if unit_info.is_translated() {
                counters.translated_unit_count += 1;
            }

            if unit_info.is_proofread {
                counters.proofread_unit_count += 1;
            }

            counters
        })
}

// Returns an unrecoverable error for a corrupt Unit chain.
fn corrupt_unit_chain_err() -> BaseError {
    BaseError::Unrecoverable {
        message: "persisted Unit chain is corrupt".to_string(),
    }
}

// Finds the position of an id in the ordered list.
fn find_order_pos(ordered_ids: &[&str], id: &str) -> Option<usize> {
    ordered_ids
        .iter()
        .enumerate()
        .find_map(|(pos, candidate)| (*candidate == id).then_some(pos))
}

// Moves an id to a new position in the ordered list by setting its next_id.
fn move_order<'a>(
    ordered_ids: &mut Vec<&'a str>,
    id: &'a str,
    next_id: Option<&str>,
) -> BaseRest<()> {
    //
    let Some(pos) = find_order_pos(ordered_ids, id) else {
        //
        let error_message = trl("error-invalid-unit-oper");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            error_message = %error_message,
            unit_id = %id,
            next_unit_id = ?next_id,
            order_count = ordered_ids.len(),
            operation = "move",
            stage = "move_order",
            "expected error: invalid unit operation",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: error_message,
        });
    };

    let id = ordered_ids.remove(pos);

    let pos = match next_id {
        //
        Some(next_id) => {
            find_order_pos(ordered_ids, next_id).ok_or_else(|| {
                //
                let error_message = trl("error-invalid-unit-oper");

                tracing::warn!(
                    error_variant = ?ExpectedVariant::Args,
                    error_message = %error_message,
                    unit_id = %id,
                    next_unit_id = %next_id,
                    order_count = ordered_ids.len(),
                    operation = "move",
                    stage = "move_order",
                    "expected error: invalid unit operation",
                );

                BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: error_message,
                }
            })?
        }

        None => ordered_ids.len(),
    };

    ordered_ids.insert(pos, id);

    accept(())
}
