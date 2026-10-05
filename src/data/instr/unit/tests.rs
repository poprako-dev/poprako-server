#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use super::*;

use serde_json::json;

use crate::model::write::unit::UnitEdit;

#[test]
#[expect(
    clippy::panic,
    reason = "This test fails explicitly when an expected fixture variant or assertion is violated"
)]
fn patch_fields_distinguish_missing_null_and_value() {
    //
    let edits = serde_json::from_value::<Vec<UnitEditInstr>>(json!([
        {
            "edit": "patch",
            "id": "unit-1"
        },
        {
            "edit": "patch",
            "id": "unit-1",
            "next_id": { "type": "clear" },
            "translation": { "type": "clear" },
            "revision": { "type": "clear" }
        },
        {
            "edit": "patch",
            "id": "unit-1",
            "next_id": { "type": "assign", "value": "unit-2" },
            "translation": { "type": "assign", "value": { "translated_text": "translated" } },
            "revision": { "type": "assign", "value": { "is_proofread": true, "proofread_text": "proofread" } }
        }
    ]))
    .unwrap();

    let edits =
        into_unit_edits(edits, "editor-1", |_| "unused".to_string()).unwrap();

    let UnitEdit::Save {
        next_id,
        translation,
        revision,
        ..
    } = edits.first().unwrap()
    else {
        panic!("patch must become Save");
    };

    assert!(matches!(next_id, Patch::Skip));

    assert!(matches!(translation, Patch::Skip));

    assert!(matches!(revision, Patch::Skip));

    let UnitEdit::Save {
        next_id,
        translation,
        revision,
        ..
    } = edits.get(1).unwrap()
    else {
        panic!("patch must become Save");
    };

    assert!(matches!(next_id, Patch::Clear));

    assert!(matches!(translation, Patch::Clear));

    assert!(matches!(revision, Patch::Clear));

    let UnitEdit::Save {
        next_id,
        translation,
        revision,
        ..
    } = edits.get(2).unwrap()
    else {
        panic!("patch must become Save");
    };

    assert!(matches!(
        next_id,
        Patch::Assign { value: id } if id == "unit-2"
    ));

    assert!(matches!(
        translation,
        Patch::Assign { value }
            if value.last_translator_id == "editor-1"
    ));

    assert!(matches!(
        revision,
        Patch::Assign { value }
            if value.last_proofreader_id == "editor-1"
    ));
}

#[test]
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
fn create_requires_structure_and_resolves_local_references() {
    //
    let missing_coord = serde_json::from_value::<Vec<UnitEditInstr>>(json!([
        {
            "edit": "create",
            "local_id": "local-a",
            "is_bubble": true
        }
    ]));

    assert!(missing_coord.is_err());

    let edits = serde_json::from_value::<Vec<UnitEditInstr>>(json!([
        {
            "edit": "create",
            "local_id": "local-a",
            "next_id": "local-b",
            "is_bubble": true,
            "coord": {"x_coord": 1.0, "y_coord": 2.0}
        },
        {
            "edit": "create",
            "local_id": "local-b",
            "is_bubble": false,
            "coord": {"x_coord": 3.0, "y_coord": 4.0}
        },
        {
            "edit": "patch",
            "id": "local-a",
            "translation": { "type": "assign", "value": { "translated_text": "text" } }
        }
    ]))
    .unwrap();

    let mut next_id = 0;

    let edits = into_unit_edits(edits, "editor-1", |_| {
        //
        next_id += 1;

        format!("server-{}", next_id)
    })
    .unwrap();

    assert!(matches!(
        edits.first().unwrap(),
        UnitEdit::Create {
            id,
            next_id: Some(anchor),
            ..
        } if id == "server-1" && anchor == "server-2"
    ));

    assert!(matches!(
        edits.get(2).unwrap(),
        UnitEdit::Save { id, .. } if id == "server-1"
    ));
}

#[test]
fn conversion_rejects_duplicate_local_ids() {
    //
    let duplicate = serde_json::from_value::<Vec<UnitEditInstr>>(json!([
        {
            "edit": "create",
            "local_id": "local-a",
            "is_bubble": true,
            "coord": {"x_coord": 1.0, "y_coord": 2.0}
        },
        {
            "edit": "create",
            "local_id": "local-a",
            "is_bubble": false,
            "coord": {"x_coord": 3.0, "y_coord": 4.0}
        }
    ]))
    .unwrap();

    assert!(into_unit_edits(duplicate, "editor-1", |_| String::new()).is_err());
}

#[test]
fn transform_conversion_accepts_literal_pairs() {
    //
    let instr = serde_json::from_value::<TransformChapterUnitsInstr>(json!({
        "part": "translated_text",
        "units": [{
            "unit_id": "unit-1",
            "transforms": [
                {"origin": "abc", "target": "def"},
                {"origin": "def", "target": ""}
            ]
        }]
    }))
    .unwrap();

    let transforms = into_unit_transforms(instr.units).unwrap();

    assert_eq!(transforms.len(), 1);

    assert_eq!(
        transforms
            .first()
            .unwrap()
            .transforms
            .get(1)
            .unwrap()
            .target,
        ""
    );
}

#[test]
fn transform_conversion_rejects_duplicate_ids_origins_and_empty_origin() {
    //
    let duplicate_ids = vec![
        UnitTransformInstr {
            unit_id: "unit-1".to_string(),
            transforms: vec![UnitTextTransformInstr {
                origin: "abc".to_string(),
                target: "def".to_string(),
            }],
        },
        UnitTransformInstr {
            unit_id: "unit-1".to_string(),
            transforms: vec![UnitTextTransformInstr {
                origin: "ghi".to_string(),
                target: "jkl".to_string(),
            }],
        },
    ];

    assert!(into_unit_transforms(duplicate_ids).is_err());

    let invalid_pairs = vec![UnitTransformInstr {
        unit_id: "unit-1".to_string(),
        transforms: vec![
            UnitTextTransformInstr {
                origin: "abc".to_string(),
                target: "def".to_string(),
            },
            UnitTextTransformInstr {
                origin: "abc".to_string(),
                target: "ghi".to_string(),
            },
        ],
    }];

    assert!(into_unit_transforms(invalid_pairs).is_err());

    let empty_origin = vec![UnitTransformInstr {
        unit_id: "unit-1".to_string(),
        transforms: vec![UnitTextTransformInstr {
            origin: String::new(),
            target: "removed".to_string(),
        }],
    }];

    assert!(into_unit_transforms(empty_origin).is_err());
}

// flagged(unit_instr)(positive): create defaults and nullable patches preserve existing clients.
#[test]
fn flagged_transport_defaults_and_types() {
    let create = json!({"edit":"create","local_id":"a","is_bubble":true,
        "coord":{"x_coord":0.0,"y_coord":0.0}});

    let instr =
        serde_json::from_value::<UnitEditInstr>(create.clone()).unwrap();

    assert!(matches!(
        instr,
        UnitEditInstr::Create {
            is_flagged: false,
            ..
        }
    ));

    for value in [json!(false), json!(true)] {
        let mut payload = create.clone();

        payload
            .as_object_mut()
            .unwrap()
            .insert("is_flagged".into(), value.clone());

        let instr = serde_json::from_value::<UnitEditInstr>(payload).unwrap();

        assert!(
            matches!(instr, UnitEditInstr::Create { is_flagged, .. } if Some(is_flagged) == value.as_bool())
        );
    }

    for value in [json!(null), json!(false), json!(true)] {
        let instr = serde_json::from_value::<UnitEditInstr>(json!({
            "edit":"patch","id":"a","is_flagged":value,
        }))
        .unwrap();

        assert!(
            matches!(instr, UnitEditInstr::Patch { is_flagged, .. } if is_flagged == value.as_bool())
        );
    }

    for value in [json!("true"), json!(1), json!({"type":"clear"})] {
        assert!(
            serde_json::from_value::<UnitEditInstr>(json!({
                "edit":"patch","id":"a","is_flagged":value,
            }))
            .is_err()
        );
    }
}

// unit_patch(negative): legacy bare values must never be advertised as valid patches.
#[test]
fn patch_rejects_bare_values_and_preserves_null_or_explicit_skip() {
    for (field, value) in [
        ("next_id", json!("unit-2")),
        ("translation", json!({"translated_text": "text"})),
        ("revision", json!({"is_proofread": false})),
    ] {
        let mut payload = json!({"edit": "patch", "id": "unit-1"});

        payload.as_object_mut().unwrap().insert(field.into(), value);

        assert!(serde_json::from_value::<UnitEditInstr>(payload).is_err());
    }

    for value in [json!(null), json!({"type": "skip"})] {
        let payload = json!({
            "edit": "patch", "id": "unit-1",
            "next_id": value, "translation": value, "revision": value,
        });

        let instr = serde_json::from_value::<UnitEditInstr>(payload).unwrap();

        assert!(matches!(
            instr,
            UnitEditInstr::Patch {
                next_id: Patch::Skip,
                translation: Patch::Skip,
                revision: Patch::Skip,
                ..
            }
        ));
    }
}
