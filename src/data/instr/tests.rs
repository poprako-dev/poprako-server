//! Sensitive fields keep their transport values while Debug follows build policy.

use std::fmt::Debug;

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::data::instr::announcement::{
    CreateAnnouncementInstr, UpdateAnnouncementInfoInstr,
};
use crate::data::instr::assignment_invitation::JoinAssignmentInvitationInstr;
use crate::data::instr::auth::{LoginAuthInstr, RegisterAuthInstr};
use crate::data::instr::comment::CreateCommentInstr;
use crate::data::instr::issue::ImportChapterIssuesInstr;
use crate::data::instr::member::JoinTeamInstr;
use crate::data::instr::term::{CreateTermInstr, UpdateTermInfoInstr};
use crate::data::instr::termbase_port::ImportTermbaseInstr;
use crate::data::instr::unit::{
    SavePageUnitEditsInstr, UnitRevisionInstr, UnitTranslationInstr,
};
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

    check_sensitive::<RegisterAuthInstr>(
        json!({"qid": "safe-id", "nickname": "safe-name", "password": "private-sentinel", "code": "private-sentinel"}),
    )?;

    check_sensitive::<UpdateUserPasswordInstr>(
        json!({"current_password": "private-sentinel", "new_password": "private-sentinel"}),
    )?;

    check_sensitive::<JoinTeamInstr>(json!({"code": "private-sentinel"}))?;

    check_sensitive::<JoinAssignmentInvitationInstr>(
        json!({"code": "private-sentinel"}),
    )?;

    Ok(())
}

// Ordinary business text is visible independently of the assertion mode.
fn check_business<T: Debug + DeserializeOwned>(
    value: Value,
) -> serde_json::Result<T> {
    //
    let instr: T = serde_json::from_value(value)?;

    for output in [format!("{instr:?}"), format!("{instr:#?}")] {
        //
        assert!(output.contains("business-sentinel"));

        assert!(!output.contains("[REDACTED]"));
    }

    Ok(instr)
}

#[test]
fn business_content_and_nested_imports_remain_visible() -> serde_json::Result<()>
{
    //
    check_business::<CreateAnnouncementInstr>(
        json!({"team_id": "team-1", "title": "business-sentinel", "content": "business-sentinel"}),
    )?;

    check_business::<UpdateAnnouncementInfoInstr>(
        json!({"id": "announcement-1", "title": "business-sentinel", "content": "business-sentinel"}),
    )?;

    check_business::<CreateCommentInstr>(
        json!({"team_id": "team-1", "content": "business-sentinel"}),
    )?;

    check_business::<CreateTermInstr>(
        json!({"termbase_id": "termbase-1", "source": "business-sentinel", "targets": ["business-sentinel"], "comment": "business-sentinel"}),
    )?;

    check_business::<UpdateTermInfoInstr>(
        json!({"id": "term-1", "source": "business-sentinel", "targets": ["business-sentinel"], "comment": "business-sentinel"}),
    )?;

    check_business::<ImportTermbaseInstr>(
        json!({"name": "business-sentinel", "description": "business-sentinel", "terms": [{"source": "business-sentinel", "targets": ["business-sentinel"], "comment": "business-sentinel"}]}),
    )?;

    check_business::<ImportChapterIssuesInstr>(
        json!({"pages": [{"page_artwork_id": "page-1", "issues": [{"variant": "business-sentinel", "layer_name": "business-sentinel", "rect": null, "note": "business-sentinel"}]}]}),
    )?;

    Ok(())
}

#[test]
fn nested_unit_debug_keeps_business_content_and_serialization()
-> serde_json::Result<()> {
    //
    let translation_json = json!({"translated_text": "business-sentinel"});

    let translation =
        check_business::<UnitTranslationInstr>(translation_json.clone())?;

    assert_eq!(serde_json::to_value(translation)?, translation_json);

    let revision_json =
        json!({"is_proofread": true, "proofread_text": "business-sentinel"});

    let revision = check_business::<UnitRevisionInstr>(revision_json.clone())?;

    assert_eq!(serde_json::to_value(revision)?, revision_json);

    let save = check_business::<SavePageUnitEditsInstr>(json!({
        "save_id": "save-1",
        "page_id": "page-1",
        "edits": [{
            "edit": "patch",
            "id": "unit-1",
            "translation": {"type": "assign", "value": translation_json},
            "revision": {"type": "assign", "value": revision_json},
        }],
    }))?;

    for edit in save.edits {
        let output = format!("{edit:?}");

        assert!(output.contains("business-sentinel"));
    }

    Ok(())
}
