// create(create)(positive): creation normalizes text and increments term_count.
// create(create)(negative): admin without a translation role cannot create a term.
// create(create)(negative): duplicate normalized targets are rejected.
// create(create)(negative): an empty target list is rejected in the business layer.
// list_infos(list_infos)(positive): fuzzy source does not search targets or comments.
// update_info(update_info)(positive): update replaces fields and touches the parent.
// delete(delete)(positive): deletion removes the term and decrements term_count.
// create(create)(negative): the 101st term is rejected without persistence.

use super::*;

use time::OffsetDateTime;

use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::termbase::TermbaseInfo;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::fixture::team;
use crate::test_util::{assert_expected_message, assert_expected_variant};
use crate::value::role::{RoleField, RoleMask};

fn token(user_id: &str) -> UserToken {
    // Build a token fixture for term-level authorisation and mutation checks.
    UserToken {
        user_id: user_id.into(),
    }
}

fn member(user_id: &str, roles: RoleMask) -> MemberInfo {
    // Build a team member fixture with explicit role bits.
    MemberInfo {
        id: format!("member-{}", user_id),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: "team-1".into(),
        user: None,
        team: None,
        roles,
    }
}

fn termbase() -> TermbaseInfo {
    //
    // Build a shared glossary container fixture.
    let time = OffsetDateTime::now_utc();

    TermbaseInfo {
        id: "termbase-1".into(),
        team_id: Some("team-1".into()),
        comic_id: None,
        name: "Glossary".into(),
        description: None,
        term_count: 0,
        creator_id: "user-1".into(),
        created_at: time,
        updated_at: time,
    }
}

fn seed_scope(mock: &Mock, role: RoleField) {
    //
    mock.seed_team(team("team-1", "Team", "Desc"));

    mock.seed_member(member("user-1", RoleMask::from(role)));

    mock.seed_termbase(termbase());
}

fn create_instr() -> CreateTermInstr {
    CreateTermInstr {
        termbase_id: "termbase-1".into(),
        source: "  Source  ".into(),
        targets: vec![" Target A ".into(), "Target B".into()],
        comment: Some("   ".into()),
    }
}

#[tokio::test]
async fn create_normalizes_and_increments_count() {
    //
    let mock = Mock::new();

    seed_scope(&mock, RoleField::TRANSLATOR);

    let val = create((&mock, &mock), token("user-1"), create_instr())
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.terms.len(), 1);

    assert_eq!(snapshot.terms[0].id, val.id);

    assert_eq!(snapshot.terms[0].source, "Source");

    assert_eq!(snapshot.terms[0].targets, ["Target A", "Target B"]);

    assert!(snapshot.terms[0].comment.is_none());

    assert_eq!(snapshot.termbases[0].term_count, 1);
}

#[tokio::test]
async fn create_rejects_admin_without_translation_role() {
    //
    let mock = Mock::new();

    seed_scope(&mock, RoleField::ADMIN);

    let error = create((&mock, &mock), token("user-1"), create_instr())
        .await
        .unwrap_err();

    assert_expected_message(
        error,
        ExpectedVariant::Perm,
        "error-team-translator-or-proofreader-required",
    );

    let snapshot = mock.snapshot();

    assert!(snapshot.terms.is_empty());

    assert_eq!(snapshot.termbases[0].term_count, 0);
}

#[tokio::test]
async fn create_rejects_duplicate_normalized_targets() {
    //
    let mock = Mock::new();

    seed_scope(&mock, RoleField::TRANSLATOR);

    let instr = CreateTermInstr {
        targets: vec!["Target".into(), " target ".into()],
        ..create_instr()
    };

    let error = create((&mock, &mock), token("user-1"), instr)
        .await
        .unwrap_err();

    assert_expected_variant(error, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert!(snapshot.terms.is_empty());

    assert_eq!(snapshot.termbases[0].term_count, 0);
}

#[tokio::test]
async fn create_rejects_empty_targets() {
    //
    let mock = Mock::new();

    seed_scope(&mock, RoleField::TRANSLATOR);

    let instr = CreateTermInstr {
        targets: Vec::new(),
        ..create_instr()
    };

    let error = create((&mock, &mock), token("user-1"), instr)
        .await
        .unwrap_err();

    assert_expected_variant(error, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert!(snapshot.terms.is_empty());

    assert_eq!(snapshot.termbases[0].term_count, 0);
}

#[tokio::test]
async fn create_rejects_term_over_capacity() {
    //
    let mock = Mock::new();

    seed_scope(&mock, RoleField::TRANSLATOR);

    mock.state.lock().unwrap().termbases[0].term_count = 100;

    let error = create((&mock, &mock), token("user-1"), create_instr())
        .await
        .unwrap_err();

    assert_expected_message(
        error,
        ExpectedVariant::Args,
        "error-termbase-term-limit",
    );

    let snapshot = mock.snapshot();

    assert!(snapshot.terms.is_empty());

    assert_eq!(snapshot.termbases[0].term_count, 100);
}

#[tokio::test]
async fn list_infos_searches_only_source() {
    //
    let mock = Mock::new();

    seed_scope(&mock, RoleField::PROOFREADER);

    create((&mock, &mock), token("user-1"), create_instr())
        .await
        .unwrap();

    let target_instr = ListTermInfosInstr {
        termbase_id: "termbase-1".into(),
        fuzzy_source: Some("target".into()),
        offset: 0,
        limit: 20,
    };

    let target_infos = list_infos((&mock,), token("user-1"), target_instr)
        .await
        .unwrap();

    assert!(target_infos.is_empty());

    let source_instr = ListTermInfosInstr {
        termbase_id: "termbase-1".into(),
        fuzzy_source: Some("source".into()),
        offset: 0,
        limit: 20,
    };

    let source_infos = list_infos((&mock,), token("user-1"), source_instr)
        .await
        .unwrap();

    assert_eq!(source_infos.len(), 1);
}

#[tokio::test]
async fn update_replaces_fields_and_touches_parent() {
    //
    let mock = Mock::new();

    seed_scope(&mock, RoleField::TRANSLATOR);

    let val = create((&mock, &mock), token("user-1"), create_instr())
        .await
        .unwrap();

    let before = mock.snapshot().termbases[0].updated_at;

    let instr = UpdateTermInfoInstr {
        id: val.id.clone(),
        source: " Updated ".into(),
        targets: vec![" New ".into()],
        comment: Some(" Comment ".into()),
    };

    update_info((&mock, &mock), token("user-1"), instr)
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.terms[0].source, "Updated");

    assert_eq!(snapshot.terms[0].targets, ["New"]);

    assert_eq!(snapshot.terms[0].comment.as_deref(), Some("Comment"));

    assert!(snapshot.termbases[0].updated_at >= before);
}

#[tokio::test]
async fn delete_removes_term_and_decrements_count() {
    //
    let mock = Mock::new();

    seed_scope(&mock, RoleField::TRANSLATOR);

    let val = create((&mock, &mock), token("user-1"), create_instr())
        .await
        .unwrap();

    delete((&mock, &mock), token("user-1"), val.id)
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.terms.is_empty());

    assert_eq!(snapshot.termbases[0].term_count, 0);
}
