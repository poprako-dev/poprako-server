//! RDB-backed Unit edit application and sequence mutation.

use std::collections::HashSet;

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::model::read::proj::unit::{UnitCountMetrics, UnitInfo, UnitOrder};
use crate::model::write::unit::UnitEdit;
use crate::part_impl::repo::rdb_impl::entity::unit::{
    UnitAspectRow, UnitEntryRow,
};
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{
    f_id, f_page_id, t_unit,
};
use crate::part_impl::repo::rdb_impl::unit::sequence::list_infos;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::RdbConn;
use crate::shared::result::diesel;
use crate::util::Patch;
use crate::value::unit::MAX_PAGE_UNIT_COUNT;

/// Finds the position of an ID in the ordered list.
pub fn find_order_pos(ordered_ids: &[&str], id: &str) -> Option<usize> {
    //
    ordered_ids
        .iter()
        .enumerate()
        .find_map(|(pos, cand)| (*cand == id).then_some(pos))
}

/// Moves an ID to a new position by assigning its requested successor.
pub fn move_order<'a>(
    ordered_ids: &mut Vec<&'a str>,
    id: &'a str,
    next_id: Option<&str>,
) -> BaseRest<()> {
    //
    let Some(pos) = find_order_pos(ordered_ids, id) else {
        //
        let err_message = trl("error-invalid-unit-oper");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            unit_id = %id,
            next_unit_id = ?next_id,
            order_count = ordered_ids.len(),
            operation = "move",
            stage = "move_order",
            "expected error: invalid unit operation",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    };

    let next_pos = match next_id {
        //
        Some(next_id) => {
            //
            let next_pos = ordered_ids.iter().enumerate().find_map(
                |(cand_pos, cand_id)| {
                    (cand_pos != pos && *cand_id == next_id).then_some(cand_pos)
                },
            );

            let Some(next_pos) = next_pos else {
                //
                let err_message = trl("error-invalid-unit-oper");

                tracing::warn!(
                    error_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    unit_id = %id,
                    next_unit_id = %next_id,
                    order_count = ordered_ids.len(),
                    operation = "move",
                    stage = "move_order",
                    "expected error: invalid unit operation",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            };

            Some(next_pos)
        }

        None => None,
    };

    debug_assert!(pos < ordered_ids.len(), "the located Unit must exist");

    let moved_id = ordered_ids.remove(pos);

    debug_assert_eq!(moved_id, id, "the located Unit must match the move ID");

    let insert_pos = next_pos.map_or(ordered_ids.len(), |next_pos| {
        //
        debug_assert_ne!(
            next_pos, pos,
            "a Unit cannot use itself as its successor"
        );

        next_pos.saturating_sub(usize::from(next_pos > pos))
    });

    debug_assert!(
        insert_pos <= ordered_ids.len(),
        "the insertion position must remain inside the Unit sequence"
    );

    ordered_ids.insert(insert_pos, moved_id);

    accept(())
}

/// Applies normalized Unit edits to produce the final ordered ID list.
pub fn apply_order_edits<'a>(
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
            let err_message = trl("error-invalid-unit-oper");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                unit_id = %id,
                existing_order_count = ordered_ids.len(),
                operation = "create",
                stage = "apply_order_edits",
                "expected error: invalid unit operation",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        ordered_ids.push(id);

        hidden_ids.remove(id.as_str());
    }

    for edit in edits {
        //
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

                    Patch::Assign { value: next_id } => {
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

    if visible_count > MAX_PAGE_UNIT_COUNT {
        //
        let err_message = trl("error-invalid-unit-oper");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            visible_count,
            max_visible_count = MAX_PAGE_UNIT_COUNT,
            operation = "reorder",
            stage = "apply_order_edits",
            "expected error: invalid unit operation",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    accept(ordered_ids)
}

/// Computes visible Unit counters from the ordered Unit info list.
pub fn count_infos(unit_infos: &[UnitInfo]) -> UnitCountMetrics {
    //
    unit_infos
        .iter()
        .filter(|unit_info| unit_info.hidden_at.is_none())
        .fold(UnitCountMetrics::default(), |mut counters, unit_info| {
            //
            counters.total += 1;

            if unit_info.is_translated() {
                counters.translated += 1;
            }

            if unit_info.is_proofread {
                counters.proofread += 1;
            }

            counters
        })
}

/// Applies normalized Unit edits and returns the latest visible counters.
#[instrument(level = "info", skip_all)]
pub async fn apply_edits(
    conn: &mut RdbConn,
    page_id: &str,
    orders: &[UnitOrder],
    edits: &[UnitEdit],
) -> BaseRest<UnitCountMetrics> {
    //
    let ordered_ids = apply_order_edits(orders, edits)?;

    apply_edit_rows(conn, page_id, edits).await?;

    apply_order_rows(conn, page_id, orders, &ordered_ids).await?;

    let unit_infos = list_infos(conn, page_id).await?;

    accept(count_infos(&unit_infos))
}

// Persist Unit creation, deletion, and content changes in request order.
async fn apply_edit_rows(
    conn: &mut RdbConn,
    page_id: &str,
    edits: &[UnitEdit],
) -> BaseRest<()> {
    //
    for edit in edits {
        //
        let (id, operation, affected) = match edit {
            //
            UnitEdit::Create { id, .. } => {
                //
                let Some(entry) = UnitEntryRow::from_edit(page_id, edit) else {
                    //
                    let err_message = trl("error-invalid-unit-oper");

                    tracing::warn!(
                        error_variant = ?ExpectedVariant::Args,
                        err_message = %err_message,
                        page_id = %page_id,
                        unit_id = %id,
                        operation = "create",
                        stage = "apply_edits",
                        "expected error: invalid unit operation",
                    );

                    return Err(BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: err_message,
                    });
                };

                diesel::insert_into(t_unit)
                    .values(entry)
                    .execute(conn)
                    .await
                    .map_err(diesel)?;

                (id, "create", None)
            }

            UnitEdit::Delete { id } => {
                //
                let affected = diesel::update(
                    t_unit.filter(f_page_id.eq(page_id)).filter(f_id.eq(id)),
                )
                .set(UnitAspectRow::new().hide())
                .execute(conn)
                .await
                .map_err(diesel)?;

                (id, "delete", Some(affected))
            }

            UnitEdit::Save { id, .. } => {
                //
                let affected = diesel::update(
                    t_unit.filter(f_page_id.eq(page_id)).filter(f_id.eq(id)),
                )
                .set(UnitAspectRow::new().apply_edit(edit))
                .execute(conn)
                .await
                .map_err(diesel)?;

                (id, "save", Some(affected))
            }
        };

        let Some(affected) = affected else {
            continue;
        };

        if affected != 1 {
            //
            let err_message = trl("error-invalid-unit-oper");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                page_id = %page_id,
                unit_id = %id,
                operation,
                affected,
                stage = "apply_edits",
                "expected error: invalid unit operation",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }
    }

    accept(())
}

// Persist successor changes needed by the final normalized Unit order.
async fn apply_order_rows(
    conn: &mut RdbConn,
    page_id: &str,
    orders: &[UnitOrder],
    ordered_ids: &[&str],
) -> BaseRest<()> {
    //
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
        .set(UnitAspectRow::new().order(next_id))
        .execute(conn)
        .await
        .map_err(diesel)?;

        if affected != 1 {
            //
            let err_message = trl("error-invalid-unit-oper");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
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
                message: err_message,
            });
        }
    }

    accept(())
}
