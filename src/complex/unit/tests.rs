use super::*;

use crate::model::shared::unit::UnitCoord;
use crate::result::ExpectedVariant;

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
        vec![save("a", Patch::Assign("a".to_string()))],
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

fn assert_args(error: BaseError) {
    assert!(matches!(
        error,
        BaseError::Expected {
            variant: ExpectedVariant::Args,
            ..
        }
    ));
}
