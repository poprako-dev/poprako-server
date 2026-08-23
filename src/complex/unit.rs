//! Domain rules and perm checks for page Units.

// perm gates for Unit reads and edit fields.
mod perm;

#[cfg(test)]
mod tests;

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::model::read::proj::unit::UnitInfo;
use crate::model::shared::unit::{UnitRevision, UnitTranslation};
use crate::model::write::unit::{UnitEdit, UnitTransform};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::{Patch, next_snowflake_id};
use crate::value::chapter::Stage;
use crate::value::unit::UnitTextPart;

pub use crate::complex::unit::perm::{UnitListAccess, UnitPermComplex};

// Build the client-visible error for an invalid Unit transform.
fn invalid_unit_transform(unit_id: &str, reason: &'static str) -> BaseError {
    //
    let err_message = trl("error-invalid-unit-transform");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        unit_id,
        reason,
        "expected error: invalid unit transform",
    );

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    }
}

// Apply every non-overlapping transform against the same original text.
fn transform_text(
    original: &str,
    unit_transform: &UnitTransform,
) -> BaseRest<Option<String>> {
    //
    let mut origins = HashSet::with_capacity(unit_transform.transforms.len());

    let mut matches = Vec::new();

    for transform in &unit_transform.transforms {
        //
        if transform.origin.is_empty()
            || !origins.insert(transform.origin.as_str())
        {
            return Err(invalid_unit_transform(
                &unit_transform.unit_id,
                "invalid_origin",
            ));
        }

        if transform.origin == transform.target {
            continue;
        }

        matches.extend(original.match_indices(&transform.origin).map(
            |(start, origin)| {
                (start, start + origin.len(), transform.target.as_str())
            },
        ));
    }

    matches.sort_by_key(|text_match| (text_match.0, text_match.1));

    for pair in matches.windows(2) {
        //
        if pair[1].0 < pair[0].1 {
            //
            return Err(invalid_unit_transform(
                &unit_transform.unit_id,
                "overlapping_matches",
            ));
        }
    }

    if matches.is_empty() {
        return accept(None);
    }

    let mut transformed = String::with_capacity(original.len());

    let mut cursor = 0;

    for (start, end, target) in matches {
        //
        transformed.push_str(&original[cursor..start]);

        transformed.push_str(target);

        cursor = end;
    }

    transformed.push_str(&original[cursor..]);

    match transformed == original {
        //
        true => accept(None),

        false => accept(Some(transformed)),
    }
}

/// Pure Unit mutation and linked-list rules.
pub struct UnitComplex;

impl UnitComplex {
    /// Trims and validates a Unit search phrase.
    pub fn normalize_search_phrase(phrase: String) -> BaseRest<String> {
        //
        let phrase = phrase.trim().to_string();

        if phrase.is_empty() {
            //
            let err_message = trl("error-unit-search-phrase-required");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                "expected error: unit search phrase required",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        accept(phrase)
    }

    /// Reports whether the selected Unit text part contains a literal phrase.
    pub fn text_part_contains(
        unit_info: &UnitInfo,
        part: UnitTextPart,
        phrase: &str,
    ) -> bool {
        //
        let text = match part {
            //
            UnitTextPart::TranslatedText => &unit_info.translated_text,

            UnitTextPart::ProofreadText => &unit_info.proofread_text,
        };

        text.as_ref().is_some_and(|text| text.contains(phrase))
    }

    /// Builds one content-only edit from non-overlapping literal transforms.
    pub fn build_transform_edit(
        unit_info: &UnitInfo,
        part: UnitTextPart,
        unit_transform: &UnitTransform,
        user_id: &str,
    ) -> BaseRest<Option<UnitEdit>> {
        //
        let original = match part {
            //
            UnitTextPart::TranslatedText => {
                unit_info.translated_text.as_deref()
            }

            UnitTextPart::ProofreadText => unit_info.proofread_text.as_deref(),
        };

        let Some(original) = original else {
            return accept(None);
        };

        let transformed = transform_text(original, unit_transform)?;

        let Some(transformed) = transformed else {
            return accept(None);
        };

        let (translation, revision) = match part {
            //
            UnitTextPart::TranslatedText => (
                Patch::Assign {
                    value: UnitTranslation {
                        translated_text: transformed,
                        last_translator_id: user_id.to_string(),
                    },
                },
                Patch::Skip,
            ),

            UnitTextPart::ProofreadText => (
                Patch::Skip,
                Patch::Assign {
                    value: UnitRevision {
                        is_proofread: unit_info.is_proofread,
                        proofread_text: Some(transformed),
                        last_proofreader_id: user_id.to_string(),
                    },
                },
            ),
        };

        accept(Some(UnitEdit::Save {
            id: unit_info.id.clone(),
            next_id: Patch::Skip,
            is_bubble: None,
            coord: None,
            translation,
            revision,
        }))
    }

    /// Generates one permanent Unit ID.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Returns workflow stages triggered by submitted Unit content.
    pub fn submitted_stage_advances(edits: &[UnitEdit]) -> Vec<Stage> {
        //
        let translated = edits.iter().any(|edit| match edit {
            //
            UnitEdit::Create {
                translation: Some(translation),
                ..
            }
            | UnitEdit::Save {
                translation: Patch::Assign { value: translation },
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
                revision: Patch::Assign { value: revision },
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
                err_variant = ?ExpectedVariant::Args,
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

        // Reorder edits so Delete precedes Create and Create precedes Save.
        // This keeps every new unit ahead of its Saves: compression never
        // merges into an earlier Create, and applying the batch never targets
        // a unit before it is created.
        let delete_count = stable_prior(&mut edits, |edit| {
            matches!(edit, UnitEdit::Delete { .. })
        });

        stable_prior(&mut edits[delete_count..], |edit| {
            matches!(edit, UnitEdit::Create { .. })
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

            let mut prev = last;

            while prev > 0 {
                //
                prev -= 1;

                let action = match (&edits[last], &edits[prev]) {
                    //
                    (
                        UnitEdit::Create { id, .. }
                        | UnitEdit::Save { id, .. }
                        | UnitEdit::Delete { id },
                        UnitEdit::Delete { id: prev_id },
                    ) if id == prev_id => 1,

                    (
                        UnitEdit::Create { id, .. }
                        | UnitEdit::Save { id, .. }
                        | UnitEdit::Delete { id },
                        UnitEdit::Save { id: prev_id, .. },
                    ) if id == prev_id => 2,

                    _ => 0,
                };

                match action {
                    //
                    1 => {
                        //
                        edits.remove(prev);

                        last -= 1;
                    }

                    2 => {
                        //
                        let (head, tail) = edits.split_at_mut(last);

                        Self::merge_edits(&mut head[prev], &mut tail[0]);

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
                err_variant = ?ExpectedVariant::Args,
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
                        err_variant = ?ExpectedVariant::Args,
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
                        err_variant = ?ExpectedVariant::Args,
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
                    next_id: Patch::Assign { value: next_id },
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
                    err_variant = ?ExpectedVariant::Args,
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
        //
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
    //
    if later.is_none() {
        *later = earlier.take();
    }
}

// Replace skipped newer patch values with previous values while keeping skips in place.
fn inherit_patch<T>(earlier: &mut Patch<T>, later: &mut Patch<T>) {
    //
    if matches!(later, Patch::Skip) {
        *later = std::mem::replace(earlier, Patch::Skip);
    }
}
