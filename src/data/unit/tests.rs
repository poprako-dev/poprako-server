use super::*;

use serde_json::json;

use crate::model::write::unit::UnitEdit;

#[test]
fn patch_fields_distinguish_missing_null_and_value() {
    //
    let edits = serde_json::from_value::<Vec<UnitEditVal>>(json!([
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
        into_unit_edits(edits, "editor-1", || "unused".to_string()).unwrap();

    let UnitEdit::Save {
        next_id,
        translation,
        revision,
        ..
    } = &edits[0]
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
    } = &edits[1]
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
    } = &edits[2]
    else {
        panic!("patch must become Save");
    };

    assert!(matches!(
        next_id,
        Patch::Assign(id) if id == "unit-2"
    ));

    assert!(matches!(
        translation,
        Patch::Assign(value)
            if value.last_translator_id == "editor-1"
    ));

    assert!(matches!(
        revision,
        Patch::Assign(value)
            if value.last_proofreader_id == "editor-1"
    ));
}

#[test]
fn create_requires_structure_and_resolves_local_references() {
    //
    let missing_coord = serde_json::from_value::<Vec<UnitEditVal>>(json!([
        {
            "edit": "create",
            "local_id": "local-a",
            "is_bubble": true
        }
    ]));

    assert!(missing_coord.is_err());

    let edits = serde_json::from_value::<Vec<UnitEditVal>>(json!([
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

    let edits = into_unit_edits(edits, "editor-1", || {
        //
        next_id += 1;

        format!("server-{}", next_id)
    })
    .unwrap();

    assert!(matches!(
        &edits[0],
        UnitEdit::Create {
            id,
            next_id: Some(anchor),
            ..
        } if id == "server-1" && anchor == "server-2"
    ));

    assert!(matches!(
        &edits[2],
        UnitEdit::Save { id, .. } if id == "server-1"
    ));
}

#[test]
fn conversion_rejects_duplicate_local_ids() {
    //
    let duplicate = serde_json::from_value::<Vec<UnitEditVal>>(json!([
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

    assert!(into_unit_edits(duplicate, "editor-1", String::new).is_err());
}
