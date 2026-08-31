//! Domain rules and perm checks for page Units.

/// Perm gates for Unit reads and edit fields.
pub mod perm;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use poprako_util::i18n::{trl, trl_kv};

use crate::model::read::proj::unit::UnitInfo;
use crate::model::shared::unit::{UnitRevision, UnitTranslation};
use crate::model::write::unit::{UnitEdit, UnitTransform};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::{Patch, next_snowflake_id};
use crate::value::chapter::stage::Stage;
use crate::value::unit::{MAX_UNIT_EDIT_COUNT, UnitTextPart};

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

// Build the client-visible error for an invalid Unit operation.
const fn invalid_unit_oper(err_message: String) -> BaseError {
    //
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

    for (left_match, right_match) in matches.iter().zip(matches.iter().skip(1))
    {
        //
        if right_match.0 < left_match.1 {
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
        let unchanged = original.get(cursor..start).ok_or_else(|| {
            //
            BaseError::Unrecoverable {
                message: "Unit transform text boundary is invalid".into(),
            }
        })?;

        transformed.push_str(unchanged);

        transformed.push_str(target);

        cursor = end;
    }

    let unchanged =
        original
            .get(cursor..)
            .ok_or_else(|| BaseError::Unrecoverable {
                message: "Unit transform final text boundary is invalid".into(),
            })?;

    transformed.push_str(unchanged);

    accept((transformed != original).then_some(transformed))
}

/// Pure Unit mutation and linked-list rules.
pub struct UnitComplex;

impl UnitComplex {
    /// Trims and validates a Unit search phrase.
    pub fn normalize_search_phrase(phrase: &str) -> BaseRest<String> {
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
        edits: Vec<UnitEdit>,
    ) -> BaseRest<Vec<UnitEdit>> {
        //
        if !(1..=MAX_UNIT_EDIT_COUNT).contains(&edits.len()) {
            //
            let args = HashMap::from([
                ("min_count".into(), 1_usize.into()),
                ("max_count".into(), MAX_UNIT_EDIT_COUNT.into()),
            ]);

            let err_message = trl_kv("error-invalid-unit-edit-count", &args);

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                edit_count = edits.len(),
                max_edit_count = MAX_UNIT_EDIT_COUNT,
                base_id_count = base_ids.len(),
                "expected error: unit edit count is invalid",
            );

            return Err(invalid_unit_oper(err_message));
        }

        let (delete_edits, non_delete_edits) =
            edits.into_iter().partition::<Vec<_>, _>(|edit| {
                matches!(edit, UnitEdit::Delete { .. })
            });

        let (create_edits, save_edits) =
            non_delete_edits.into_iter().partition::<Vec<_>, _>(|edit| {
                matches!(edit, UnitEdit::Create { .. })
            });

        let mut edits = delete_edits
            .into_iter()
            .chain(create_edits)
            .chain(save_edits)
            .collect::<Vec<_>>();

        Self::compress_edits(&mut edits);

        Self::final_validate_edits(base_ids, &edits)?;

        accept(edits)
    }

    // Merge adjacent or contradictory edit operations into canonical forms.
    fn compress_edits(edits: &mut Vec<UnitEdit>) {
        //
        let mut edit_slots = std::mem::take(edits)
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>();

        let mut remaining_slots = edit_slots.as_mut_slice();

        while let Some((later_slot, earlier_slots)) =
            remaining_slots.split_last_mut()
        {
            //
            let Some(later_edit) = later_slot.as_mut() else {
                //
                remaining_slots = earlier_slots;

                continue;
            };

            for earlier_slot in earlier_slots.iter_mut().rev() {
                //
                let action = match (&*later_edit, earlier_slot.as_ref()) {
                    //
                    (
                        UnitEdit::Create { id, .. }
                        | UnitEdit::Save { id, .. }
                        | UnitEdit::Delete { id },
                        Some(UnitEdit::Delete { id: prev_id }),
                    ) if id == prev_id => 1,

                    (
                        UnitEdit::Create { id, .. }
                        | UnitEdit::Save { id, .. }
                        | UnitEdit::Delete { id },
                        Some(UnitEdit::Save { id: prev_id, .. }),
                    ) if id == prev_id => 2,

                    _ => 0,
                };

                match action {
                    //
                    1 => {
                        earlier_slot.take();
                    }

                    2 => {
                        //
                        let Some(mut earlier_edit) = earlier_slot.take() else {
                            continue;
                        };

                        Self::merge_edits(&mut earlier_edit, later_edit);
                    }

                    _ => {}
                }
            }

            remaining_slots = earlier_slots;
        }

        edits.extend(edit_slots.into_iter().flatten());
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

            return Err(invalid_unit_oper(err_message));
        }

        edits.iter().try_for_each(|edit| {
            Self::validate_edit(&base_ids, &deleted_ids, &created_ids, edit)
        })
    }

    // Merge a prior save edit into a later save edit for the same unit.
    fn merge_edits(earlier: &mut UnitEdit, later: &mut UnitEdit) {
        //
        let (
            UnitEdit::Save {
                id: _,
                next_id: earlier_next_id,
                is_bubble: earlier_is_bubble,
                coord: earlier_coord,
                translation: earlier_translation,
                revision: earlier_revision,
            },
            UnitEdit::Save {
                id: _,
                next_id: later_next_id,
                is_bubble: later_is_bubble,
                coord: later_coord,
                translation: later_translation,
                revision: later_revision,
            },
        ) = (earlier, later)
        else {
            return;
        };

        inherit_option(earlier_is_bubble, later_is_bubble);

        inherit_option(earlier_coord, later_coord);

        inherit_patch(earlier_next_id, later_next_id);

        inherit_patch(earlier_translation, later_translation);

        inherit_patch(earlier_revision, later_revision);
    }

    // Validate one edit target and its optional next pointer.
    fn validate_edit(
        base_ids: &HashSet<&str>,
        deleted_ids: &HashSet<&str>,
        created_ids: &HashSet<&str>,
        edit: &UnitEdit,
    ) -> BaseRest<()> {
        //
        match edit {
            //
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

                return Err(invalid_unit_oper(err_message));
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

                return Err(invalid_unit_oper(err_message));
            }

            _ => {}
        }

        let Some((id, next_id)) = (match edit {
            //
            UnitEdit::Create {
                id,
                next_id: Some(next_id),
                ..
            }
            | UnitEdit::Save {
                id,
                next_id: Patch::Assign { value: next_id },
                ..
            } => Some((id, next_id)),

            _ => None,
        }) else {
            return accept(());
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

            return Err(invalid_unit_oper(err_message));
        }

        accept(())
    }
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
