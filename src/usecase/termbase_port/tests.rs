// import(import)(positive): team import creates a portable termbase and terms.
// import(import)(positive): force merge updates metadata and merges targets.
// import(import)(negative): same-name import without force is atomic and rejected.
// import(import)(negative): duplicate normalized sources are rejected before persistence.
// export(export)(positive): export omits persistence metadata and orders terms.

use super::*;

use time::OffsetDateTime;

use crate::data::instr::termbase_port::{ImportTermInstr, ImportTermbaseInstr};
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::term::TermInfo;
use crate::model::read::proj::termbase::TermbaseInfo;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::fixture::team;
use crate::test_util::{assert_expected_message, assert_expected_variant};
use crate::value::role::{RoleField, RoleMask};

fn member(user_id: &str) -> MemberInfo {
    //
    MemberInfo {
        id: format!("member-{}", user_id),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: "team-1".into(),
        user: None,
        team: None,
        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

fn termbase(term_count: usize) -> TermbaseInfo {
    //
    let time = OffsetDateTime::now_utc();

    TermbaseInfo {
        id: "termbase-1".into(),
        team_id: Some("team-1".into()),
        comic_id: None,
        name: "Glossary".into(),
        description: Some("Old description".into()),
        term_count,
        creator_id: "user-original".into(),
        created_at: time,
        updated_at: time,
    }
}

fn term(id: &str, source: &str, targets: &[&str]) -> TermInfo {
    //
    let time = OffsetDateTime::now_utc();

    TermInfo {
        id: id.into(),
        termbase_id: "termbase-1".into(),
        source: source.into(),
        targets: targets.iter().map(|target| (*target).into()).collect(),
        comment: Some("Old comment".into()),
        creator_id: "user-original".into(),
        created_at: time,
        updated_at: time,
    }
}

fn token() -> UserToken {
    UserToken {
        user_id: "user-1".into(),
    }
}

fn import_instr(
    name: &str,
    terms: Vec<ImportTermInstr>,
) -> ImportTermbaseInstr {
    ImportTermbaseInstr {
        name: name.into(),
        description: Some("New description".into()),
        terms,
    }
}

fn import_term(source: &str, targets: &[&str]) -> ImportTermInstr {
    ImportTermInstr {
        source: source.into(),
        targets: targets.iter().map(|target| (*target).into()).collect(),
        comment: Some("New comment".into()),
    }
}

fn seed_scope(mock: &Mock) {
    //
    mock.seed_team(team("team-1", "Team", "Description"));

    mock.seed_member(member("user-1"));
}

#[tokio::test]
async fn import_creates_team_termbase_and_terms() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    let instr = import_instr(
        "  Imported  ",
        vec![import_term("Beta", &["B"]), import_term("Alpha", &["A"])],
    );

    let val = import(
        (&mock, &mock),
        token(),
        TermbaseScope::Team {
            team_id: "team-1".into(),
        },
        false,
        instr,
    )
    .await
    .unwrap();

    assert!(val.created);

    assert_eq!(val.created_term_count, 2);

    assert_eq!(val.merged_term_count, 0);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.termbases[0].name, "Imported");

    assert_eq!(snapshot.termbases[0].term_count, 2);

    assert_eq!(snapshot.terms.len(), 2);
}

#[tokio::test]
async fn import_force_merge_updates_metadata_and_merges_targets() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_termbase(termbase(1));

    mock.seed_term(term("term-1", "Source", &["Old"]));

    let instr = import_instr(
        " glossary ",
        vec![
            import_term(" source ", &[" old ", "New"]),
            import_term("Added", &["Added target"]),
        ],
    );

    let val = import(
        (&mock, &mock),
        token(),
        TermbaseScope::Team {
            team_id: "team-1".into(),
        },
        true,
        instr,
    )
    .await
    .unwrap();

    assert!(!val.created);

    assert_eq!(val.created_term_count, 1);

    assert_eq!(val.merged_term_count, 1);

    let snapshot = mock.snapshot();

    assert_eq!(
        snapshot.termbases[0].description.as_deref(),
        Some("New description")
    );

    assert_eq!(snapshot.termbases[0].term_count, 2);

    let merged_term = snapshot
        .terms
        .iter()
        .find(|term_info| term_info.id == "term-1")
        .unwrap();

    assert_eq!(merged_term.source, "source");

    assert_eq!(merged_term.targets, ["Old", "New"]);

    assert_eq!(merged_term.comment.as_deref(), Some("New comment"));

    assert_eq!(merged_term.creator_id, "user-original");
}

#[tokio::test]
async fn import_rejects_same_name_without_force_atomically() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_termbase(termbase(0));

    let error = import(
        (&mock, &mock),
        token(),
        TermbaseScope::Team {
            team_id: "team-1".into(),
        },
        false,
        import_instr("GLOSSARY", vec![import_term("Added", &["Target"])]),
    )
    .await
    .unwrap_err();

    assert_expected_message(
        error,
        ExpectedVariant::Args,
        "error-already-exists",
    );

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.termbases.len(), 1);

    assert!(snapshot.terms.is_empty());
}

#[tokio::test]
async fn import_rejects_duplicate_normalized_sources() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    let instr = import_instr(
        "Glossary",
        vec![
            import_term("Source", &["A"]),
            import_term(" source ", &["B"]),
        ],
    );

    let error = import(
        (&mock, &mock),
        token(),
        TermbaseScope::Team {
            team_id: "team-1".into(),
        },
        false,
        instr,
    )
    .await
    .unwrap_err();

    assert_expected_variant(error, ExpectedVariant::Args);

    assert!(mock.snapshot().termbases.is_empty());
}

#[tokio::test]
async fn export_orders_portable_terms_without_metadata() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_termbase(termbase(2));

    mock.seed_term(term("term-2", "Beta", &["B"]));

    mock.seed_term(term("term-1", "Alpha", &["A"]));

    let val = export((&mock, &mock), token(), "termbase-1".into())
        .await
        .unwrap();

    assert_eq!(val.name, "Glossary");

    assert_eq!(val.terms.len(), 2);

    assert_eq!(val.terms[0].source, "Alpha");

    assert_eq!(val.terms[1].source, "Beta");
}
