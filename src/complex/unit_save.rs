//! Pure Unit persistence planning and save token validation.

#[cfg(test)]
// Sparse persistence plan regression tests.
mod tests;

use std::collections::{BTreeMap, HashMap};

use poprako_util::i18n::trl;

use crate::complex::unit as unit_complex;
use crate::model::read::proj::unit::UnitOrder;
use crate::model::write::unit::{UnitEdit, UnitWriteChange, UnitWritePlan};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::Patch;

/// Reports an invalid save identity without exposing payloads.
pub fn invalid_save() -> BaseError {
    //
    let msg = trl("error-invalid-unit-oper");

    tracing::warn!(err_variant = ?ExpectedVariant::Args, err_msg = %msg,
    "invalid Unit save identity or payload");

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        msg,
    }
}

/// Plans final inserts and sparse writes without loading persisted Unit content.
/// Callers supply normalized edits and orders from their locked transaction.
pub fn plan_writes<'a>(
    orders: &'a [UnitOrder],
    edits: &'a [UnitEdit],
) -> BaseRest<UnitWritePlan<'a>> {
    //
    let sequence = unit_complex::plan_edit_sequence(orders, edits)?;

    let orders_by_id = orders
        .iter()
        .map(|order| (order.id.as_str(), order))
        .collect::<HashMap<_, _>>();

    let mut creates = BTreeMap::new();

    for edit in edits {
        //
        let UnitEdit::Create { id, .. } = edit else {
            continue;
        };

        let mut create = edit.clone();

        if let UnitEdit::Create { next_id, .. } = &mut create {
            *next_id = sequence.next_id(id)?.map(str::to_owned);
        }

        creates.insert(id.as_str(), create);
    }

    let mut changes = BTreeMap::new();

    for edit in edits {
        //
        let (id, is_hidden) = match edit {
            //
            UnitEdit::Create { .. } => continue,

            UnitEdit::Delete { id } => (id.as_str(), true),

            UnitEdit::Save { id, .. } => {
                //
                if let Some(create) = creates.get_mut(id.as_str()) {
                    //
                    patch_create(create, edit)?;

                    continue;
                }

                (id.as_str(), false)
            }
        };

        let order = orders_by_id.get(id).ok_or_else(invalid_plan)?;

        let change = UnitWriteChange {
            next_id: Patch::Skip,
            is_hidden: (order.is_hidden != is_hidden).then_some(is_hidden),
            edit: has_content(edit).then_some(edit),
        };

        if changes.insert(id, change).is_some() {
            return Err(invalid_plan());
        }
    }

    for successor in sequence.changed_successors() {
        //
        changes.entry(successor.id()).or_default().next_id = successor
            .next_id()
            .map_or(Patch::Clear, |value| Patch::Assign { value });
    }

    let mut skipped_ids = Vec::new();

    changes.retain(|id, change| {
        //
        if change.is_empty() {
            //
            skipped_ids.push(*id);

            return false;
        }

        true
    });

    accept(UnitWritePlan {
        creates: creates.into_values().collect(),
        changes,
        skipped_ids,
    })
}

// Folds content into a new row only after all sequence moves have been planned.
fn patch_create(create: &mut UnitEdit, edit: &UnitEdit) -> BaseRest<()> {
    //
    let (
        UnitEdit::Create {
            is_bubble,
            is_flagged,
            coord,
            typography_note,
            translation_note,
            translation,
            revision,
            ..
        },
        UnitEdit::Save {
            is_bubble: bubble,
            is_flagged: flagged,
            coord: position,
            translation: translated,
            revision: proofread,
            typography_note: typography_annotation,
            translation_note: translation_annotation,
            ..
        },
    ) = (create, edit)
    else {
        return Err(invalid_plan());
    };

    if let Some(bubble) = bubble {
        *is_bubble = *bubble;
    }

    if let Some(flagged) = flagged {
        *is_flagged = *flagged;
    }

    if let Some(position) = position {
        *coord = position.clone();
    }

    patch_initial(translation, translated);

    patch_initial(revision, proofread);

    patch_initial(typography_note, typography_annotation);

    patch_initial(translation_note, translation_annotation);

    accept(())
}

// Distinguishes submitted content from order-only or empty saves.
const fn has_content(edit: &UnitEdit) -> bool {
    //
    match edit {
        //
        UnitEdit::Save {
            is_bubble,
            is_flagged,
            coord,
            typography_note,
            translation_note,
            translation,
            revision,
            ..
        } => {
            //
            is_bubble.is_some()
                || is_flagged.is_some()
                || coord.is_some()
                || !matches!(translation, Patch::Skip)
                || !matches!(revision, Patch::Skip)
                || !typography_note.is_skip()
                || !translation_note.is_skip()
        }

        _ => false,
    }
}

// Reports a mismatch between normalized edits and their persisted sequence.
fn invalid_plan() -> BaseError {
    //
    tracing::error!("normalized Unit edits disagree with persistence plan");

    BaseError::Unrecoverable {
        msg: "normalized Unit edits disagree with persistence plan".into(),
    }
}

// Applies a submitted field to an initial creation value.
fn patch_initial<T>(initial: &mut Option<T>, patch: &Patch<T>)
where
    T: Clone,
{
    //
    match patch {
        //
        Patch::Skip => {}

        Patch::Clear => *initial = None,

        Patch::Assign { value } => *initial = Some(value.clone()),
    }
}
