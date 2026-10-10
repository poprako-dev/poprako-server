#![allow(
    clippy::unwrap_used,
    reason = "Transport fixture assertions fail immediately on violated contracts"
)]

//! Issue input decoding preserves readable names and rejects obsolete fields.

use super::IssueInstr;

// issue_input(IssueInstr)(positive): readable names remain verbatim and are optional for whole-page issues.
#[test]
fn readable_layer_name_is_preserved() {
    for layer_name in [None, Some("  他们两个…  ")] {
        let value = serde_json::json!({
            "variant": "字号错误",
            "layer_name": layer_name,
            "note": "eg2"
        });

        let instr = serde_json::from_value::<IssueInstr>(value).unwrap();

        assert_eq!(instr.layer_name.as_deref(), layer_name);
    }

    let unnamed = serde_json::json!({ "variant": "其他", "note": "整页说明" });

    assert!(
        serde_json::from_value::<IssueInstr>(unnamed)
            .unwrap()
            .layer_name
            .is_none()
    );
}

// issue_input(IssueInstr)(negative): obsolete layer paths cannot silently become unnamed issues.
#[test]
fn obsolete_layer_field_is_rejected() {
    let legacy = serde_json::json!({
        "variant": "字号错误",
        "layer_path": "0.2.5",
        "note": "eg2"
    });

    assert!(serde_json::from_value::<IssueInstr>(legacy).is_err());
}
