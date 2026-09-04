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

// Build one persisted Unit sequence node.
fn order(id: &str, next_id: Option<&str>, is_hidden: bool) -> UnitOrder {
    UnitOrder {
        id: id.to_string(),
        next_id: next_id.map(str::to_string),
        is_hidden,
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
fn edit_sequence_plan_combines_create_restore_delete_and_moves() {
    //
    let orders = vec![
        order("a", Some("b"), false),
        order("b", Some("c"), true),
        order("c", None, false),
    ];

    let edits = vec![
        create("d", Some("b".to_string())),
        save("b", Patch::Clear),
        UnitEdit::Delete {
            id: "c".to_string(),
        },
    ];

    let Ok(plan) = UnitComplex::plan_edit_sequence(&orders, &edits) else {
        assert!(false, "valid Unit edits must produce a sequence plan");

        return;
    };

    assert_eq!(plan.ordered_ids(), &["a", "d", "c", "b"]);

    assert!(matches!(plan.next_id("d"), Ok(Some("c"))));

    assert_eq!(plan.visible_count(), 3);

    let successor_changes = plan
        .changed_successors()
        .iter()
        .map(|change| (change.id(), change.next_id()))
        .collect::<Vec<_>>();

    assert_eq!(
        successor_changes,
        [("a", Some("d")), ("b", None), ("c", Some("b")),],
    );
}

#[test]
fn edit_sequence_plan_handles_a_long_tombstone_chain_linearly() {
    //
    let ids = (0..600)
        .map(|index| format!("unit-{index:03}"))
        .collect::<Vec<_>>();

    let orders = ids
        .iter()
        .enumerate()
        .map(|(index, id)| UnitOrder {
            id: id.clone(),
            next_id: ids.get(index + 1).cloned(),
            is_hidden: true,
        })
        .collect::<Vec<_>>();

    let Some(last_id) = ids.last() else {
        assert!(false, "the fixture must contain a last Unit");

        return;
    };

    let Some(first_id) = ids.first() else {
        assert!(false, "the fixture must contain a first Unit");

        return;
    };

    let edits = vec![save(
        last_id,
        Patch::Assign {
            value: first_id.clone(),
        },
    )];

    let Ok(plan) = UnitComplex::plan_edit_sequence(&orders, &edits) else {
        assert!(false, "a valid long tombstone chain must remain editable");

        return;
    };

    assert_eq!(plan.ordered_ids().first(), Some(&last_id.as_str()));

    assert_eq!(plan.visible_count(), 1);
}

#[test]
fn edit_sequence_plan_rejects_visible_overflow_and_corrupt_order() {
    //
    let ids = (0..100)
        .map(|index| format!("unit-{index:03}"))
        .collect::<Vec<_>>();

    let orders = ids
        .iter()
        .enumerate()
        .map(|(index, id)| UnitOrder {
            id: id.clone(),
            next_id: ids.get(index + 1).cloned(),
            is_hidden: false,
        })
        .collect::<Vec<_>>();

    let overflow_edits = vec![create("overflow", None)];

    let overflow = UnitComplex::plan_edit_sequence(&orders, &overflow_edits);

    assert!(matches!(
        overflow,
        Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            ..
        }),
    ));

    let corrupt_orders = vec![order("a", None, false), order("b", None, false)];

    assert!(matches!(
        UnitComplex::plan_edit_sequence(&corrupt_orders, &[]),
        Err(BaseError::Unrecoverable { .. }),
    ));
}

#[test]
fn search_phrase_trims_unicode_and_accepts_one_character() {
    //
    let phrase =
        UnitComplex::normalize_search_phrase(" 译文甲 ".into()).unwrap();

    assert_eq!(phrase, "译文甲");

    assert_eq!(
        UnitComplex::normalize_search_phrase(" 日 ".into()).unwrap(),
        "日",
    );

    assert_args(UnitComplex::normalize_search_phrase("".into()).unwrap_err());

    assert_args(
        UnitComplex::normalize_search_phrase(" \u{2003}\n ".into())
            .unwrap_err(),
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
