//! Domain rules and perm checks for page Units.

// Linear-time linked-sequence planning for Unit edits.
mod edit;
// Literal text transformation for existing Unit content.
mod text_transform;

/// Perm gates for Unit reads and edit fields.
pub mod perm;
/// Reconstruction of complete persisted Unit chains.
pub mod sequence;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use poprako_util::i18n::{trl, trl_kv};

use crate::complex::unit::text_transform as unit_text_transform_complex;
use crate::model::read::proj::unit::{UnitInfo, UnitOrder};
use crate::model::write::unit::{UnitEdit, UnitTransform};
use crate::result::{BaseRest, ExpectedVariant, accept};
use crate::util::{Patch, next_snowflake_id};
use crate::value::chapter::stage::Stage;
use crate::value::unit::{MAX_UNIT_EDIT_COUNT, UnitTextPart};

/// Treats blank annotations as absent while preserving meaningful formatting.
pub fn normalize_note(note: Option<String>) -> Option<String> {
    note.filter(|value| !value.trim().is_empty())
}

/// Borrows the trimmed, validated slice of a Unit search phrase.
pub fn normalize_search_phrase(phrase: &str) -> BaseRest<&str> {
    //
    let phrase = phrase.trim();

    if phrase.is_empty() {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-unit-search-phrase-required");
            "unit search phrase required",
        });
    }

    accept(phrase)
}

/// Builds one content-only edit from non-overlapping literal transforms.
pub fn build_transform_edit(
    unit_info: &UnitInfo,
    part: UnitTextPart,
    unit_transform: &UnitTransform,
    user_id: &str,
) -> BaseRest<Option<UnitEdit>> {
    //
    unit_text_transform_complex::build_transform_edit(
        unit_info,
        part,
        unit_transform,
        user_id,
    )
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
        } => !translation.trimmed_translated_text().is_empty(),

        _ => false,
    });

    let has_typography_note = edits.iter().any(|edit| match edit {
        //
        UnitEdit::Create {
            typography_note: Some(note),
            ..
        }
        | UnitEdit::Save {
            typography_note: Patch::Assign { value: note },
            ..
        } => !note.trim().is_empty(),

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

    if translated || has_typography_note {
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

        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl_kv("error-invalid-unit-edit-count", &args);
            edit_count = edits.len(),
            max_edit_count = MAX_UNIT_EDIT_COUNT,
            base_id_count = base_ids.len(),
            "unit edit count is invalid",
        });
    }

    let (delete_edits, non_delete_edits) = edits
        .into_iter()
        .partition::<Vec<_>, _>(|edit| matches!(edit, UnitEdit::Delete { .. }));

    let (create_edits, save_edits) = non_delete_edits
        .into_iter()
        .partition::<Vec<_>, _>(|edit| matches!(edit, UnitEdit::Create { .. }));

    let mut edits = delete_edits
        .into_iter()
        .chain(create_edits)
        .chain(save_edits)
        .collect::<Vec<_>>();

    for edit in &mut edits {
        //
        match edit {
            //
            UnitEdit::Create {
                typography_note,
                translation_note,
                ..
            } => {
                //
                *typography_note = normalize_note(typography_note.take());

                *translation_note = normalize_note(translation_note.take());
            }

            UnitEdit::Save {
                typography_note,
                translation_note,
                ..
            } => {
                //
                normalize_note_patch(typography_note);

                normalize_note_patch(translation_note);
            }

            UnitEdit::Delete { .. } => {}
        }
    }

    compress_edits(&mut edits);

    final_validate_edits(base_ids, &edits)?;

    accept(edits)
}

/// Plans one normalized Unit edit batch against the persisted chain.
pub fn plan_edit_sequence<'a>(
    orders: &'a [UnitOrder],
    edits: &'a [UnitEdit],
) -> BaseRest<edit::UnitEditSequencePlan<'a>> {
    edit::UnitEditSequencePlan::build(orders, edits)
}

// Converts blank assignments to explicit clears without changing skipped patches.
fn normalize_note_patch(note: &mut Patch<String>) {
    //
    if matches!(note, Patch::Assign { value } if value.trim().is_empty()) {
        *note = Patch::Clear;
    }
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

                    merge_edits(&mut earlier_edit, later_edit);
                }

                _ => {}
            }
        }

        remaining_slots = earlier_slots;
    }

    edits.extend(edit_slots.into_iter().flatten());
}

// Validate cross-edit consistency and pointer validity.
fn final_validate_edits(base_ids: &[&str], edits: &[UnitEdit]) -> BaseRest<()> {
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
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-invalid-unit-oper");
            edit_count = edits.len(),
            base_id_count = base_ids.len(),
            create_count,
            created_id_count = created_ids.len(),
            "unit create edits are inconsistent",
        });
    }

    edits.iter().try_for_each(|edit| {
        validate_edit(&base_ids, &deleted_ids, &created_ids, edit)
    })
}

// Merge a prior save edit into a later save edit for the same Unit.
fn merge_edits(earlier: &mut UnitEdit, later: &mut UnitEdit) {
    //
    let (
        UnitEdit::Save {
            next_id: earlier_next_id,
            is_bubble: earlier_is_bubble,
            is_flagged: earlier_is_flagged,
            coord: earlier_coord,
            translation: earlier_translation,
            revision: earlier_revision,
            typography_note: earlier_typography_note,
            translation_note: earlier_translation_note,
            ..
        },
        UnitEdit::Save {
            next_id: later_next_id,
            is_bubble: later_is_bubble,
            is_flagged: later_is_flagged,
            coord: later_coord,
            translation: later_translation,
            revision: later_revision,
            typography_note: later_typography_note,
            translation_note: later_translation_note,
            ..
        },
    ) = (earlier, later)
    else {
        return;
    };

    inherit_option(earlier_is_bubble, later_is_bubble);

    inherit_option(earlier_is_flagged, later_is_flagged);

    inherit_option(earlier_coord, later_coord);

    inherit_patch(earlier_next_id, later_next_id);

    inherit_patch(earlier_translation, later_translation);

    inherit_patch(earlier_revision, later_revision);

    inherit_patch(earlier_typography_note, later_typography_note);

    inherit_patch(earlier_translation_note, later_translation_note);
}

// Validate one edit target and its optional next pointer.
fn validate_edit(
    base_ids: &HashSet<&str>,
    deleted_ids: &HashSet<&str>,
    created_ids: &HashSet<&str>,
    edit: &UnitEdit,
) -> BaseRest<()> {
    //
    let invalid_target = match edit {
        //
        UnitEdit::Delete { id } if !base_ids.contains(id.as_str()) => {
            Some((id, "delete"))
        }

        UnitEdit::Save { id, .. }
            if !base_ids.contains(id.as_str())
                && !created_ids.contains(id.as_str()) =>
        {
            Some((id, "save"))
        }

        _ => None,
    };

    if let Some((id, operation)) = invalid_target {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-invalid-unit-oper");
            unit_id = %id,
            operation,
            "unit edit target is invalid",
        });
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
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-invalid-unit-oper");
            unit_id = %id,
            next_unit_id = %next_id,
            "unit next pointer is invalid",
        });
    }

    accept(())
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
