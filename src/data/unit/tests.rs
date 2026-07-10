// unit_diff_data_into_model(UnitDiffData::into_model)(positive): minimal create defaults optional state and content.
// unit_diff_data_into_model(UnitDiffData::into_model)(positive): create and save preserve supplied complete content.
// unit_diff_data_rejects_legacy_or_mixed_identifiers(UnitDiffData)(negative): legacy and mixed create-save identifiers are rejected.

use super::*;

use crate::model::unit::UnitOper;

#[test]
fn unit_diff_data_into_model_defaults_minimal_create() {
    //
    let value = serde_json::json!({
        "page_id": "page-1",
        "opers": [{
            "oper": "create",
            "local_id": "local-1",
            "is_bubble": true,
            "x_coord": 1.0,
            "y_coord": 2.0
        }]
    });

    let unit_diff_data: UnitDiffData =
        serde_json::from_value(value).ok().unwrap();

    let unit_diff = unit_diff_data.into_model().unwrap();

    match &unit_diff.opers[0] {
        //
        UnitOper::Create {
            id: local_id,
            payload,
            before_id,
        } => {
            //
            assert_eq!(local_id, "local-1");

            assert!(before_id.is_none());

            assert!(payload.is_bubble);

            assert!(!payload.is_proofread);

            assert!(payload.translated_text.is_none());

            assert!(payload.last_translator_id.is_none());

            assert!(payload.proofread_text.is_none());

            assert!(payload.last_proofreader_id.is_none());
        }

        UnitOper::Save { .. } | UnitOper::Delete { .. } => {
            panic!("expected create oper");
        }
    }
}

#[test]
fn unit_diff_data_into_model_preserves_create_and_save_content() {
    //
    let value = serde_json::json!({
        "page_id": "page-1",
        "opers": [
            {
                "oper": "create",
                "local_id": "local-1",
                "before_id": "unit-a",
                "is_bubble": true,
                "is_proofread": true,
                "x_coord": 1.0,
                "y_coord": 2.0,
                "translated_text": "translated",
                "last_translator_id": "user-1",
                "proofread_text": "proofread",
                "last_proofreader_id": "user-2"
            },
            {
                "oper": "save",
                "id": "unit-a",
                "is_bubble": false,
                "is_proofread": false,
                "x_coord": 3.0,
                "y_coord": 4.0,
                "translated_text": null,
                "last_translator_id": null,
                "proofread_text": null,
                "last_proofreader_id": null
            }
        ]
    });

    let unit_diff_data: UnitDiffData =
        serde_json::from_value(value).ok().unwrap();

    let unit_diff = unit_diff_data.into_model().unwrap();

    match &unit_diff.opers[0] {
        //
        UnitOper::Create {
            payload, before_id, ..
        } => {
            //
            assert_eq!(before_id.as_deref(), Some("unit-a"));

            assert!(payload.is_proofread);

            assert_eq!(payload.translated_text.as_deref(), Some("translated"));

            assert_eq!(payload.last_translator_id.as_deref(), Some("user-1"));

            assert_eq!(payload.proofread_text.as_deref(), Some("proofread"));

            assert_eq!(payload.last_proofreader_id.as_deref(), Some("user-2"));
        }

        UnitOper::Save { .. } | UnitOper::Delete { .. } => {
            panic!("expected create oper");
        }
    }

    match &unit_diff.opers[1] {
        //
        UnitOper::Save { id, payload, .. } => {
            //
            assert_eq!(id, "unit-a");

            assert!(!payload.is_bubble);

            assert!(payload.translated_text.is_none());
        }

        UnitOper::Create { .. } | UnitOper::Delete { .. } => {
            panic!("expected save oper");
        }
    }
}

#[test]
fn unit_diff_data_rejects_legacy_or_mixed_identifiers() {
    //
    let invalid_opers = [
        serde_json::json!({
            "oper": "save",
            "local_id": "local-1",
            "is_bubble": true,
            "is_proofread": false,
            "x_coord": 1.0,
            "y_coord": 2.0
        }),
        serde_json::json!({
            "oper": "create",
            "local_id": "local-1",
            "id": "unit-a",
            "is_bubble": true,
            "x_coord": 1.0,
            "y_coord": 2.0
        }),
        serde_json::json!({
            "oper": "save",
            "is_bubble": true,
            "is_proofread": false,
            "x_coord": 1.0,
            "y_coord": 2.0
        }),
    ];

    for invalid_oper in invalid_opers {
        //
        let value = serde_json::json!({
            "page_id": "page-1",
            "opers": [invalid_oper]
        });

        let result = serde_json::from_value::<UnitDiffData>(value);

        assert!(result.is_err());
    }
}
