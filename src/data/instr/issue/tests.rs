#![allow(
    clippy::unwrap_used,
    reason = "Transport fixture assertions fail immediately on violated contracts"
)]

//! Issue input decoding rejects retired layer fields.

use super::IssueInstr;

// issue_input(IssueInstr)(positive): whole-page issues need only a category and note.
#[test]
fn whole_page_issue_decodes() {
    let value = serde_json::json!({ "variant": "其他", "note": "整页说明" });

    let instr = serde_json::from_value::<IssueInstr>(value).unwrap();

    assert_eq!(instr.variant, "其他");

    assert!(instr.rect.is_none());
}

// issue_input(IssueInstr)(negative): retired layer fields cannot be silently ignored.
#[test]
fn retired_layer_fields_are_rejected() {
    for field in ["layer_name", "layer_path"] {
        let mut value =
            serde_json::json!({ "variant": "字号错误", "note": "eg2" });

        value
            .as_object_mut()
            .unwrap()
            .insert(field.into(), serde_json::json!("retired"));

        assert!(serde_json::from_value::<IssueInstr>(value).is_err());
    }
}
