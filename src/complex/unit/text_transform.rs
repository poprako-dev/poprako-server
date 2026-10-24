//! Literal text transformation for existing Unit content.

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::model::read::proj::unit::UnitInfo;
use crate::model::shared::unit::{UnitRevision, UnitTranslation};
use crate::model::write::unit::{UnitEdit, UnitTransform};
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::util::Patch;
use crate::value::unit::UnitTextPart;

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
        UnitTextPart::TranslatedText => unit_info.translated_text.as_deref(),

        UnitTextPart::ProofreadText => unit_info.proofread_text.as_deref(),
    };

    let Some(original) = original else {
        return accept!(None);
    };

    let transformed = transform_text(original, unit_transform)?;

    let Some(transformed) = transformed else {
        return accept!(None);
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

    accept!(Some(UnitEdit::Save {
        id: unit_info.id.clone(),
        next_id: Patch::Skip,
        is_bubble: None,
        is_flagged: None,
        coord: None,
        typography_note: Patch::Skip,
        translation_note: Patch::Skip,
        translation,
        revision,
    }))
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
        return accept!(None);
    }

    let mut transformed = String::with_capacity(original.len());

    let mut cursor = 0;

    for (start, end, target) in matches {
        //
        let unchanged = original.get(cursor..start).ok_or_else(|| {
            //
            unrecoverable_error! {
                message: "Unit transform text boundary is invalid".into();
                unit_id = %unit_transform.unit_id,
                text_byte_len = original.len(),
                cursor,
                start,
                end,
                "Unit transform text boundary is invalid",
            }
        })?;

        transformed.push_str(unchanged);

        transformed.push_str(target);

        cursor = end;
    }

    let unchanged = original.get(cursor..).ok_or_else(|| {
        //
        unrecoverable_error! {
            message: "Unit transform final text boundary is invalid".into();
            unit_id = %unit_transform.unit_id,
            text_byte_len = original.len(),
            cursor,
            "Unit transform final text boundary is invalid",
        }
    })?;

    transformed.push_str(unchanged);

    accept!((transformed != original).then_some(transformed))
}

// Build the client-visible error for an invalid Unit transform.
fn invalid_unit_transform(unit_id: &str, reason: &'static str) -> BaseError {
    //
    expected_error! {
        ExpectedVariant::Args,
        message: trl("error-invalid-unit-transform");
        unit_id,
        reason,
        "invalid unit transform",
    }
}
