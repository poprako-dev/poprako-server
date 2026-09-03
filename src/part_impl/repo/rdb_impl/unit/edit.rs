//! RDB-backed Unit edit application and sequence mutation.

use std::collections::HashSet;

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::model::read::proj::unit::{
    UnitCountMetrics, UnitOrder, has_unit_text,
};
use crate::model::write::unit::UnitEdit;
use crate::part_impl::repo::rdb_impl::entity::unit::{
    UnitAspectRow, UnitEntryRow,
};
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{
    f_hidden_at, f_id, f_is_proofread, f_page_id, f_proofread_text,
    f_translated_text, t_unit,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
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

    apply_edit_rows(conn, page_id, edits, &ordered_ids).await?;

    apply_order_rows(conn, page_id, orders, edits, &ordered_ids).await?;

    count_visible_units(conn, page_id).await
}

// Persist Unit creation, deletion, and content changes in request order.
async fn apply_edit_rows(
    conn: &mut RdbConn,
    page_id: &str,
    edits: &[UnitEdit],
    ordered_ids: &[&str],
) -> BaseRest<()> {
    //
    let delete_ids = edits
        .iter()
        .filter_map(|edit| match edit {
            //
            UnitEdit::Delete { id } => Some(id.as_str()),

            UnitEdit::Create { .. } | UnitEdit::Save { .. } => None,
        })
        .collect::<Vec<_>>();

    apply_delete_rows(conn, page_id, &delete_ids).await?;

    let create_entries = edits
        .iter()
        .filter_map(|edit| {
            //
            let id = match edit {
                //
                UnitEdit::Create { id, .. } => Some(id),

                UnitEdit::Save { .. } | UnitEdit::Delete { .. } => None,
            }?;

            let next_id = ordered_ids
                .iter()
                .position(|ordered_id| *ordered_id == id)
                .and_then(|index| ordered_ids.get(index + 1))
                .copied();

            UnitEntryRow::from_edit(page_id, edit, next_id)
        })
        .collect::<Vec<_>>();

    apply_create_rows(conn, page_id, &create_entries).await?;

    for edit in edits {
        //
        if let UnitEdit::Save { id, .. } = edit {
            //
            let affected = diesel::update(
                t_unit.filter(f_page_id.eq(page_id)).filter(f_id.eq(id)),
            )
            .set(UnitAspectRow::new().apply_edit(edit))
            .execute(conn)
            .await
            .map_err(diesel)?;

            ensure_affected(page_id, id, "save", affected, 1)?;
        }
    }

    accept(())
}

// Persist successor changes needed by the final normalized Unit order.
async fn apply_order_rows(
    conn: &mut RdbConn,
    page_id: &str,
    orders: &[UnitOrder],
    edits: &[UnitEdit],
    ordered_ids: &[&str],
) -> BaseRest<()> {
    //
    let created_ids = edits
        .iter()
        .filter_map(|edit| match edit {
            //
            UnitEdit::Create { id, .. } => Some(id.as_str()),

            UnitEdit::Save { .. } | UnitEdit::Delete { .. } => None,
        })
        .collect::<HashSet<_>>();

    for (index, id) in ordered_ids.iter().enumerate() {
        //
        if created_ids.contains(id) {
            continue;
        }

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

        ensure_affected(page_id, id, "reorder", affected, 1)?;
    }

    accept(())
}

// Loads only the visible Unit fields required to compute Page counters.
async fn count_visible_units(
    conn: &mut RdbConn,
    page_id: &str,
) -> BaseRest<UnitCountMetrics> {
    //
    let count_fields = t_unit
        .filter(f_page_id.eq(page_id))
        .filter(f_hidden_at.is_null())
        .select((f_translated_text, f_proofread_text, f_is_proofread))
        .load::<(Option<String>, Option<String>, bool)>(conn)
        .await
        .map_err(diesel)?;

    let counters = count_fields.into_iter().fold(
        UnitCountMetrics::default(),
        |mut counters, (translated_text, proofread_text, is_proofread)| {
            //
            counters.total += 1;

            if has_unit_text(translated_text.as_deref())
                || has_unit_text(proofread_text.as_deref())
            {
                counters.translated += 1;
            }

            if is_proofread {
                counters.proofread += 1;
            }

            counters
        },
    );

    accept(counters)
}

// Hide an exact Unit ID set in one statement.
async fn apply_delete_rows(
    conn: &mut RdbConn,
    page_id: &str,
    ids: &[&str],
) -> BaseRest<()> {
    //
    if ids.is_empty() {
        return accept(());
    }

    let affected = diesel::update(
        t_unit
            .filter(f_page_id.eq(page_id))
            .filter(f_id.eq_any(ids)),
    )
    .set(UnitAspectRow::new().hide())
    .execute(conn)
    .await
    .map_err(diesel)?;

    ensure_affected(page_id, "batch", "delete", affected, ids.len())
}

// Insert an exact Unit row set in one statement.
async fn apply_create_rows(
    conn: &mut RdbConn,
    page_id: &str,
    entries: &[UnitEntryRow<'_>],
) -> BaseRest<()> {
    //
    if entries.is_empty() {
        return accept(());
    }

    let affected = diesel::insert_into(t_unit)
        .values(entries)
        .execute(conn)
        .await
        .map_err(diesel)?;

    ensure_affected(page_id, "batch", "create", affected, entries.len())
}

// Reject a mutation whose affected row count differs from its exact input.
fn ensure_affected(
    page_id: &str,
    unit_id: &str,
    operation: &str,
    affected: usize,
    expected: usize,
) -> BaseRest<()> {
    //
    if affected == expected {
        return accept(());
    }

    let err_message = trl("error-invalid-unit-oper");

    tracing::warn!(
        error_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        page_id,
        unit_id,
        operation,
        affected,
        expected,
        stage = "apply_edits",
        "expected error: invalid unit operation",
    );

    Err(BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    })
}
