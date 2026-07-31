//! Domain rules and permission checks for page Units.

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::model::write::unit::UnitEdit;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::{Patch, next_snowflake_id};
use crate::value::chapter::Stage;

pub use perm::UnitPermComplex;

// Permission gates for Unit reads and edit fields.
mod perm;
#[cfg(test)]
mod tests;

/// Pure Unit mutation and linked-list rules.
pub struct UnitComplex;

impl UnitComplex {
    /// Generates one permanent Unit ID.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Returns workflow stages started by submitted Unit content.
    pub fn submitted_stage_starts(edits: &[UnitEdit]) -> Vec<Stage> {
        //
        let translated = edits.iter().any(|edit| match edit {
            //
            UnitEdit::Create {
                translation: Some(translation),
                ..
            }
            | UnitEdit::Save {
                translation: Patch::Assign(translation),
                ..
            } => !translation.translated_text.trim().is_empty(),

            _ => false,
        });

        let proofread = edits.iter().any(|edit| match edit {
            //
            UnitEdit::Create {
                revision: Some(revision),
                ..
            }
            | UnitEdit::Save {
                revision: Patch::Assign(revision),
                ..
            } => revision.is_proofread,

            _ => false,
        });

        let mut stages = Vec::with_capacity(2);

        if translated {
            stages.push(Stage::Translate);
        }

        if proofread {
            stages.push(Stage::Proofread);
        }

        stages
    }

    /// Normalizes one Unit edit batch against the persisted Unit IDs.
    pub fn normalize_edits(
        base_ids: &[&str],
        mut edits: Vec<UnitEdit>,
    ) -> BaseRest<Vec<UnitEdit>> {
        //
        if !(1..=100).contains(&edits.len()) {
            //
            let err_message = trl("error-invalid-unit-oper");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                edit_count = edits.len(),
                base_id_count = base_ids.len(),
                "expected error: unit edit count is invalid",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        stable_prior(&mut edits, |edit| {
            matches!(edit, UnitEdit::Delete { .. })
        });

        Self::compress_edits(&mut edits);

        Self::final_validate_edits(base_ids, &edits)?;

        accept(edits)
    }

    // Merge adjacent or contradictory edit operations into canonical forms.
    fn compress_edits(edits: &mut Vec<UnitEdit>) {
        //
        let mut last = edits.len();

        while last > 0 {
            //
            last -= 1;

            let id = match &edits[last] {
                UnitEdit::Create { id, .. }
                | UnitEdit::Save { id, .. }
                | UnitEdit::Delete { id } => id.clone(),
            };

            let mut prev = last;

            while prev > 0 {
                //
                prev -= 1;

                match &edits[prev] {
                    //
                    UnitEdit::Delete { id: prev_id } if prev_id == &id => {
                        //
                        edits.remove(prev);

                        last -= 1;
                    }

                    UnitEdit::Save { id: prev_id, .. } if prev_id == &id => {
                        //
                        {
                            let (head, tail) = edits.split_at_mut(last);

                            Self::merge_edits(&mut head[prev], &mut tail[0]);
                        }

                        edits.remove(prev);

                        last -= 1;
                    }

                    _ => {}
                }
            }
        }
    }

    // Validate cross-edit consistency: create/delete/patch order and pointer validity.
    fn final_validate_edits(
        base_ids: &[&str],
        edits: &[UnitEdit],
    ) -> BaseRest<()> {
        //
        let base_ids = base_ids.iter().copied().collect::<HashSet<_>>();

        let (deleted_ids, created_ids) = edits.iter().fold(
            (HashSet::new(), HashSet::new()),
            |(mut deleted, mut created), edit| {
                //
                match edit {
                    //
                    UnitEdit::Create { id, .. } => {
                        created.insert(id.as_str());
                    }

                    UnitEdit::Delete { id } => {
                        deleted.insert(id.as_str());
                    }

                    UnitEdit::Save { .. } => {}
                }

                (deleted, created)
            },
        );

        let create_count = edits
            .iter()
            .filter(|edit| matches!(edit, UnitEdit::Create { .. }))
            .count();

        if create_count != created_ids.len()
            || created_ids.iter().any(|id| base_ids.contains(id))
        {
            let err_message = trl("error-invalid-unit-oper");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                edit_count = edits.len(),
                base_id_count = base_ids.len(),
                create_count = create_count,
                created_id_count = created_ids.len(),
                "expected error: unit create edits are inconsistent",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        edits.iter().try_for_each(|edit| {
            //
            match edit {
                //
                UnitEdit::Create { .. } => {}

                UnitEdit::Delete { id } if !base_ids.contains(id.as_str()) => {
                    //
                    let err_message = trl("error-invalid-unit-oper");

                    tracing::warn!(
                        error_variant = ?ExpectedVariant::Args,
                        err_message = %err_message,
                        unit_id = %id,
                        operation = "delete",
                        "expected error: unit delete target is invalid",
                    );

                    return Err(BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: err_message,
                    });
                }

                UnitEdit::Save { id, .. }
                    if !base_ids.contains(id.as_str())
                        && !created_ids.contains(id.as_str()) =>
                {
                    let err_message = trl("error-invalid-unit-oper");

                    tracing::warn!(
                        error_variant = ?ExpectedVariant::Args,
                        err_message = %err_message,
                        unit_id = %id,
                        operation = "save",
                        "expected error: unit save target is invalid",
                    );

                    return Err(BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: err_message,
                    });
                }

                _ => {}
            }

            let (id, next_id) = match edit {
                //
                UnitEdit::Create {
                    id,
                    next_id: Some(next_id),
                    ..
                } => (id, next_id),

                UnitEdit::Save {
                    id,
                    next_id: Patch::Assign(next_id),
                    ..
                } => (id, next_id),

                _ => return accept(()),
            };

            if next_id == id
                || (!base_ids.contains(next_id.as_str())
                    && !created_ids.contains(next_id.as_str()))
                || deleted_ids.contains(next_id.as_str())
            {
                let err_message = trl("error-invalid-unit-oper");

                tracing::warn!(
                    error_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    unit_id = %id,
                    next_unit_id = %next_id,
                    "expected error: unit next pointer is invalid",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            }

            accept(())
        })
    }

    // Merge a prior save edit into a later save edit for the same unit.
    fn merge_edits(earlier: &mut UnitEdit, later: &mut UnitEdit) {
        //
        let (
            UnitEdit::Save {
                id: earlier_id,
                next_id: earlier_next_id,
                is_bubble: earlier_is_bubble,
                coord: earlier_coord,
                translation: earlier_translation,
                revision: earlier_revision,
            },
            UnitEdit::Save {
                id: later_id,
                next_id: later_next_id,
                is_bubble: later_is_bubble,
                coord: later_coord,
                translation: later_translation,
                revision: later_revision,
            },
        ) = (earlier, later)
        else {
            unreachable!("only Unit Save edits can be merged");
        };

        debug_assert_eq!(earlier_id, later_id);

        inherit_option(earlier_is_bubble, later_is_bubble);

        inherit_option(earlier_coord, later_coord);

        inherit_patch(earlier_next_id, later_next_id);

        inherit_patch(earlier_translation, later_translation);

        inherit_patch(earlier_revision, later_revision);
    }
}

// Move all items matching `pred` to the front while preserving stable order.
fn stable_prior<T, P>(slice: &mut [T], mut pred: P) -> usize
where
    P: FnMut(&T) -> bool,
{
    let mut tail = 0;

    for current in 0..slice.len() {
        if pred(&slice[current]) {
            //
            slice[tail..=current].rotate_right(1);

            tail += 1;
        }
    }

    tail
}

// Copy the older optional value only when the newer optional is empty.
fn inherit_option<T>(earlier: &mut Option<T>, later: &mut Option<T>) {
    if later.is_none() {
        *later = earlier.take();
    }
}

// Replace skipped newer patch values with previous values while keeping skips in place.
fn inherit_patch<T>(earlier: &mut Patch<T>, later: &mut Patch<T>) {
    if matches!(later, Patch::Skip) {
        *later = std::mem::replace(earlier, Patch::Skip);
    }
}
