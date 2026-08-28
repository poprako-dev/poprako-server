use super::*;

use time::OffsetDateTime;

use crate::model::read::proj::unit::UnitInfo;
use crate::model::shared::unit::UnitCoord;
use crate::model::write::unit::{UnitTextTransform, UnitTransform};
use crate::result::ExpectedVariant;
use crate::value::unit::UnitTextPart;

// Build a mocked save edit with a deterministic `next_id` value.
fn save(id: &str, next_id: Patch<String>) -> UnitEdit {
    UnitEdit::Save {
        id: id.to_string(),
        next_id,
        is_bubble: None,
        coord: None,
        translation: Patch::Skip,
        revision: Patch::Skip,
    }
}

// Build a mocked create edit for a brand-new unit.
fn create(id: &str, next_id: Option<String>) -> UnitEdit {
    UnitEdit::Create {
        id: id.to_string(),
        next_id,
        is_bubble: false,
        coord: UnitCoord {
            x_coord: 1.0,
            y_coord: 2.0,
        },
        translation: None,
        revision: None,
    }
}

// Build one visible Unit with both text fields for transform tests.
fn unit(translated_text: &str, proofread_text: &str) -> UnitInfo {
    //
    UnitInfo {
        id: "unit-1".to_string(),
        page_id: "page-1".to_string(),
        next_id: None,
        is_bubble: true,
        coord: UnitCoord {
            x_coord: 1.0,
            y_coord: 2.0,
        },
        translated_text: Some(translated_text.to_string()),
        last_translator_id: Some("translator-old".to_string()),
        is_proofread: true,
        proofread_text: Some(proofread_text.to_string()),
        last_proofreader_id: Some("proofreader-old".to_string()),
        hidden_at: None,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    }
}

// Build one Unit transform with literal origin-target pairs.
fn transform(pairs: &[(&str, &str)]) -> UnitTransform {
    //
    UnitTransform {
        unit_id: "unit-1".to_string(),
        transforms: pairs
            .iter()
            .map(|(origin, target)| UnitTextTransform {
                origin: (*origin).to_string(),
                target: (*target).to_string(),
            })
            .collect(),
    }
}

#[test]
fn normalize_compresses_delete_and_field_patches_into_one_save() {
    //
    let edits = vec![
        save("a", Patch::Clear),
        UnitEdit::Delete {
            id: "a".to_string(),
        },
        UnitEdit::Save {
            id: "a".to_string(),
            next_id: Patch::Skip,
            is_bubble: Some(false),
            coord: Some(UnitCoord {
                x_coord: 2.0,
                y_coord: 3.0,
            }),
            translation: Patch::Skip,
            revision: Patch::Skip,
        },
    ];

    let edits = UnitComplex::normalize_edits(&["a"], edits).unwrap();

    assert_eq!(edits.len(), 1);

    assert!(matches!(
        &edits[0],
        UnitEdit::Save {
            next_id: Patch::Clear,
            is_bubble: Some(false),
            coord: Some(_),
            ..
        }
    ));
}

#[test]
fn normalize_rejects_invalid_anchors_and_unknown_targets() {
    //
    let self_anchor = UnitComplex::normalize_edits(
        &["a"],
        vec![save(
            "a",
            Patch::Assign {
                value: "a".to_string(),
            },
        )],
    );

    assert_args(self_anchor.unwrap_err());

    let unknown = UnitComplex::normalize_edits(
        &["a"],
        vec![UnitEdit::Delete {
            id: "missing".to_string(),
        }],
    );

    assert_args(unknown.unwrap_err());
}

#[test]
fn normalize_orders_create_prior_to_save_for_the_same_unit() {
    //
    let edits = UnitComplex::normalize_edits(
        &[],
        vec![save("a", Patch::Clear), create("a", None)],
    )
    .unwrap();

    assert!(matches!(
        &edits[0],
        UnitEdit::Create { id, .. } if id == "a"
    ));

    assert!(matches!(
        &edits[1],
        UnitEdit::Save { id, .. } if id == "a"
    ));
}

#[test]
fn search_phrase_trims_unicode_accepts_one_character_and_selects_text_part() {
    //
    let phrase = UnitComplex::normalize_search_phrase(" 译文甲 ").unwrap();

    assert_eq!(phrase, "译文甲");

    let unit_info = unit("译文甲正文", "校对正文");

    assert!(UnitComplex::text_part_contains(
        &unit_info,
        UnitTextPart::TranslatedText,
        &phrase,
    ));

    assert!(!UnitComplex::text_part_contains(
        &unit_info,
        UnitTextPart::ProofreadText,
        &phrase,
    ));

    assert_eq!(UnitComplex::normalize_search_phrase(" 日 ").unwrap(), "日");

    assert_args(UnitComplex::normalize_search_phrase("").unwrap_err());

    assert_args(
        UnitComplex::normalize_search_phrase(" \u{2003}\n ").unwrap_err(),
    );
}

#[test]
fn transform_uses_original_text_without_target_cascading() {
    //
    let unit_info = unit("abc def", "proofread");

    let unit_transform = transform(&[("abc", "def"), ("def", "final")]);

    let edit = UnitComplex::build_transform_edit(
        &unit_info,
        UnitTextPart::TranslatedText,
        &unit_transform,
        "translator-new",
    )
    .unwrap()
    .unwrap();

    let UnitEdit::Save {
        translation: Patch::Assign { value },
        revision: Patch::Skip,
        ..
    } = edit
    else {
        panic!("translation transform must build one content-only Save");
    };

    assert_eq!(value.translated_text, "def final");

    assert_eq!(value.last_translator_id, "translator-new");
}

#[test]
fn transform_rejects_overlapping_original_matches() {
    //
    let unit_info = unit("abcd", "proofread");

    let unit_transform = transform(&[("abc", "first"), ("bcd", "second")]);

    let error = UnitComplex::build_transform_edit(
        &unit_info,
        UnitTextPart::TranslatedText,
        &unit_transform,
        "translator-new",
    )
    .unwrap_err();

    assert_args(error);
}

#[test]
fn proofread_transform_preserves_approval_and_updates_attribution() {
    //
    let unit_info = unit("translated", "proofread old");

    let unit_transform = transform(&[("old", "new")]);

    let edit = UnitComplex::build_transform_edit(
        &unit_info,
        UnitTextPart::ProofreadText,
        &unit_transform,
        "proofreader-new",
    )
    .unwrap()
    .unwrap();

    let UnitEdit::Save {
        translation: Patch::Skip,
        revision: Patch::Assign { value },
        ..
    } = edit
    else {
        panic!("proofread transform must build one content-only Save");
    };

    assert!(value.is_proofread);

    assert_eq!(value.proofread_text.as_deref(), Some("proofread new"));

    assert_eq!(value.last_proofreader_id, "proofreader-new");
}

// Assert that an error is an argument validation error.
fn assert_args(error: BaseError) {
    assert!(matches!(
        error,
        BaseError::Expected {
            variant: ExpectedVariant::Args,
            ..
        }
    ));
}
