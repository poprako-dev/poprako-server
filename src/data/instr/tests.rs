//! Sensitive fields keep their transport values while Debug follows build policy.

use std::fmt::Debug;

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::data::instr::assignment_invitation::JoinAssignmentInvitationInstr;
use crate::data::instr::auth::{LoginAuthInstr, RegisterAuthInstr};
use crate::data::instr::member::JoinTeamInstr;
use crate::data::instr::unit::{UnitRevisionInstr, UnitTranslationInstr};
use crate::data::instr::user::UpdateUserPasswordInstr;

// Check both compact and pretty Debug on credential instructions.
fn check_sensitive<T: Debug + DeserializeOwned>(
    value: Value,
) -> serde_json::Result<T> {
    //
    let instr: T = serde_json::from_value(value)?;

    for output in [format!("{instr:?}"), format!("{instr:#?}")] {
        //
        assert_eq!(output.contains("private-sentinel"), cfg!(debug_assertions));

        assert_eq!(output.contains("[REDACTED]"), !cfg!(debug_assertions));

        assert!(!output.contains("\"[REDACTED]\""));
    }

    Ok(instr)
}

#[test]
fn credentials_and_invitation_codes_declare_their_own_redaction()
-> serde_json::Result<()> {
    //
    let login = check_sensitive::<LoginAuthInstr>(
        json!({"qid": "safe-id", "password": "private-sentinel"}),
    )?;

    assert_eq!(login.password, "private-sentinel");

    check_sensitive::<RegisterAuthInstr>(json!({
        "qid": "safe-id",
        "nickname": "safe-name",
        "password": "private-sentinel",
        "code": "private-sentinel",
    }))?;

    check_sensitive::<UpdateUserPasswordInstr>(json!({
        "current_password": "private-sentinel",
        "new_password": "private-sentinel",
    }))?;

    check_sensitive::<JoinTeamInstr>(json!({"code": "private-sentinel"}))?;

    check_sensitive::<JoinAssignmentInvitationInstr>(
        json!({"code": "private-sentinel"}),
    )?;

    Ok(())
}

#[test]
fn nested_unit_payloads_preserve_serialization() -> serde_json::Result<()> {
    //
    let translation_json = json!({"translated_text": "business-sentinel"});

    let translation = serde_json::from_value::<UnitTranslationInstr>(
        translation_json.clone(),
    )?;

    assert_eq!(serde_json::to_value(translation)?, translation_json);

    let revision_json =
        json!({"is_proofread": true, "proofread_text": "business-sentinel"});

    let revision =
        serde_json::from_value::<UnitRevisionInstr>(revision_json.clone())?;

    assert_eq!(serde_json::to_value(revision)?, revision_json);

    Ok(())
}
