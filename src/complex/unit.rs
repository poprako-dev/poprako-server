//! Domain rules and permission checks for page Units.

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::model::write::unit::UnitEdit;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::util::{PatchField, next_snowflake_id};

pub use perm::UnitPermComplex;

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

    /// Normalizes one Unit edit batch against the persisted Unit IDs.
    pub fn normalize_edits(
        base_ids: &[&str],
        mut edits: Vec<UnitEdit>,
    ) -> BaseResult<Vec<UnitEdit>> {
        //
        if !(1..=100).contains(&edits.len()) {
            return Err(invalid_unit_edit_err());
        }

        stable_prior(&mut edits, |edit| {
            matches!(edit, UnitEdit::Delete { .. })
        });

        Self::compress_edits(&mut edits);

        Self::final_validate_edits(base_ids, &edits)?;

        accept(edits)
    }

    fn compress_edits(edits: &mut Vec<UnitEdit>) {
        //
        let mut last = edits.len();

        while last > 0 {
            //
            last -= 1;

            let id = match &edits[last] {
                UnitEdit::Delete { id } | UnitEdit::Save { id, .. } => {
                    id.clone()
                }
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

    fn final_validate_edits(
        base_ids: &[&str],
        edits: &[UnitEdit],
    ) -> BaseResult<()> {
        //
        let base_ids = base_ids.iter().copied().collect::<HashSet<_>>();

        let (deleted_ids, saved_ids) = edits.iter().fold(
            (HashSet::new(), HashSet::new()),
            |(mut deleted, mut saved), edit| {
                //
                match edit {
                    //
                    UnitEdit::Delete { id } => {
                        deleted.insert(id.as_str());
                    }

                    UnitEdit::Save { id, .. } => {
                        saved.insert(id.as_str());
                    }
                }

                (deleted, saved)
            },
        );

        edits.iter().try_for_each(|edit| {
            //
            match edit {
                //
                UnitEdit::Delete { id } if !base_ids.contains(id.as_str()) => {
                    return Err(invalid_unit_edit_err());
                }

                _ => {}
            }

            let UnitEdit::Save {
                id,
                next_id: PatchField::Assign(next_id),
                ..
            } = edit
            else {
                return accept(());
            };

            if next_id == id
                || (!base_ids.contains(next_id.as_str())
                    && !saved_ids.contains(next_id.as_str()))
                || deleted_ids.contains(next_id.as_str())
            {
                return Err(invalid_unit_edit_err());
            }

            accept(())
        })
    }
}

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

fn inherit_option<T>(earlier: &mut Option<T>, later: &mut Option<T>) {
    if later.is_none() {
        *later = earlier.take();
    }
}

fn inherit_patch<T>(earlier: &mut PatchField<T>, later: &mut PatchField<T>) {
    if matches!(later, PatchField::Skip) {
        *later = std::mem::replace(earlier, PatchField::Skip);
    }
}

fn invalid_unit_edit_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-invalid-unit-oper"),
    }
}
