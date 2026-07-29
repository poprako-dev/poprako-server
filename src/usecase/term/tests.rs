// create(create)(positive): creation normalizes text and increments term_count.
// create(create)(negative): duplicate normalized targets are rejected.
// list_infos(list_infos)(positive): fuzzy source does not search targets or comments.
// update_info(update_info)(positive): update replaces fields and touches the parent.
// delete(delete)(positive): deletion removes the term and decrements term_count.

use super::*;

use time::OffsetDateTime;

use crate::model::member::MemberInfo;
use crate::model::termbase::TermbaseInfo;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::test_util::fixture::team;
use crate::value::role::{RoleField, RoleMask};

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn member(user_id: &str, roles: RoleMask) -> MemberInfo {
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

fn seed_proofreader_scope(mock: &Mock) {
    //
    mock.seed_team(team("team-1", "Team", "Desc"));

    mock.seed_member(member("user-1", RoleMask::from(RoleField::PROOFREADER)));

    mock.seed_termbase(termbase());
}

fn create_params() -> CreateTermParams {
    CreateTermParams {
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

    seed_proofreader_scope(&mock);

    let payload = create(&mock, &mock, token("user-1"), create_params())
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.terms.len(), 1);

    assert_eq!(snapshot.terms[0].id, payload.id);

    assert_eq!(snapshot.terms[0].source, "Source");

    assert_eq!(snapshot.terms[0].targets, ["Target A", "Target B"]);

    assert!(snapshot.terms[0].comment.is_none());

    assert_eq!(snapshot.termbases[0].term_count, 1);
}

#[tokio::test]
async fn create_rejects_duplicate_normalized_targets() {
    //
    let mock = Mock::new();

    seed_proofreader_scope(&mock);

    let params = CreateTermParams {
        targets: vec!["Target".into(), " target ".into()],
        ..create_params()
    };

    let error = create(&mock, &mock, token("user-1"), params)
        .await
        .unwrap_err();

    assert_expected_variant(error, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert!(snapshot.terms.is_empty());

    assert_eq!(snapshot.termbases[0].term_count, 0);
}

#[tokio::test]
async fn list_infos_searches_only_source() {
    //
    let mock = Mock::new();

    seed_proofreader_scope(&mock);

    create(&mock, &mock, token("user-1"), create_params())
        .await
        .unwrap();

    let target_params = ListTermInfosParams {
        termbase_id: "termbase-1".into(),
        fuzzy_source: Some("target".into()),
        offset: 0,
        limit: 20,
    };

    let target_infos = list_infos(&mock, token("user-1"), target_params)
        .await
        .unwrap();

    assert!(target_infos.is_empty());

    let source_params = ListTermInfosParams {
        termbase_id: "termbase-1".into(),
        fuzzy_source: Some("source".into()),
        offset: 0,
        limit: 20,
    };

    let source_infos = list_infos(&mock, token("user-1"), source_params)
        .await
        .unwrap();

    assert_eq!(source_infos.len(), 1);
}

#[tokio::test]
async fn update_replaces_fields_and_touches_parent() {
    //
    let mock = Mock::new();

    seed_proofreader_scope(&mock);

    let payload = create(&mock, &mock, token("user-1"), create_params())
        .await
        .unwrap();

    let before = mock.snapshot().termbases[0].updated_at;

    let params = UpdateTermInfoParams {
        id: payload.id.clone(),
        source: " Updated ".into(),
        targets: vec![" New ".into()],
        comment: Some(" Comment ".into()),
    };

    update_info(&mock, &mock, token("user-1"), params)
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

    seed_proofreader_scope(&mock);

    let payload = create(&mock, &mock, token("user-1"), create_params())
        .await
        .unwrap();

    delete(&mock, &mock, token("user-1"), payload.id)
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.terms.is_empty());

    assert_eq!(snapshot.termbases[0].term_count, 0);
}
