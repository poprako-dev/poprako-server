#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// import(import)(positive): team import creates a portable termbase and terms.
// import(import)(positive): force merge updates metadata and merges targets.
// import(import)(negative): same-name import without force is atomic and rejected.
// import(import)(negative): duplicate normalized sources are rejected before persistence.
// export(export)(positive): export omits persistence metadata and orders terms.

use super::*;

use time::OffsetDateTime;

use crate::data::instr::termbase_port::{
    AppendTermbaseTermInstr, ImportTermInstr, ImportTermbaseInstr,
};
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
        id: ["member-", user_id].concat(),
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

    assert_eq!(snapshot.termbases.first().unwrap().name, "Imported");

    assert_eq!(snapshot.termbases.first().unwrap().term_count, 2);

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
        snapshot.termbases.first().unwrap().description.as_deref(),
        Some("New description")
    );

    assert_eq!(snapshot.termbases.first().unwrap().term_count, 2);

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

    assert_expected_variant(&error, ExpectedVariant::Args);

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

    assert_eq!(val.terms.first().unwrap().source, "Alpha");

    assert_eq!(val.terms.get(1).unwrap().source, "Beta");
}

// append(append)(positive): mixed batches preserve identity, metadata, preferred targets, and unchanged timestamps.
// append(append)(negative): invalid input, missing targets, permissions, and capacity fail atomically.

fn append_term(
    source: &str,
    targets: &[&str],
    comment: Option<&str>,
) -> AppendTermbaseTermInstr {
    AppendTermbaseTermInstr {
        source: source.into(),
        targets: targets.iter().map(|target| (*target).into()).collect(),
        comment: comment.map(str::to_owned),
    }
}

#[tokio::test]
async fn append_preserves_identity_and_metadata_and_counts_real_changes() {
    let mock = Mock::new();

    seed_scope(&mock);

    let original_base = termbase(2);

    let original_term = term("term-1", "Source", &["Preferred", "Second"]);

    let unchanged_term = term("term-2", "Stable", &["Stable target"]);

    mock.seed_termbase(original_base.clone());

    mock.seed_term(original_term.clone());

    mock.seed_term(unchanged_term.clone());

    let instr = AppendTermbaseInstr {
        terms: vec![
            append_term(" source ", &["second", "New"], Some("New note")),
            append_term("STABLE", &["stable target"], None),
            append_term("Added", &["New entry"], None),
        ],
    };

    let val = append((&mock, &mock), token(), original_base.id.clone(), instr)
        .await
        .unwrap();

    assert_eq!((val.added, val.updated, val.unchanged), (1, 1, 1));

    let snapshot = mock.snapshot();

    let base = snapshot.termbases.first().unwrap();

    assert_eq!(base.term_count, 3);

    assert_eq!(base.name, original_base.name);

    assert_eq!(base.description, original_base.description);

    assert_eq!(base.creator_id, original_base.creator_id);

    let merged = snapshot
        .terms
        .iter()
        .find(|term| term.id == original_term.id)
        .unwrap();

    assert_eq!(merged.source, "Source");

    assert_eq!(merged.targets, ["Preferred", "Second", "New"]);

    assert_eq!(merged.comment.as_deref(), Some("Old comment\nNew note"));

    assert_eq!(merged.creator_id, original_term.creator_id);

    assert_eq!(merged.created_at, original_term.created_at);

    let stable = snapshot
        .terms
        .iter()
        .find(|term| term.id == unchanged_term.id)
        .unwrap();

    assert_eq!(stable.updated_at, unchanged_term.updated_at);

    let added = snapshot
        .terms
        .iter()
        .find(|term| term.source == "Added")
        .unwrap();

    assert_eq!(added.creator_id, token().user_id);
}

#[tokio::test]
async fn append_empty_and_identical_batches_preserve_timestamps() {
    let mock = Mock::new();

    seed_scope(&mock);

    let original_base = termbase(1);

    let original_term = term("term-1", "Source", &["Preferred"]);

    mock.seed_termbase(original_base.clone());

    mock.seed_term(original_term.clone());

    for terms in [
        vec![],
        vec![append_term("SOURCE", &["preferred"], Some("Old comment"))],
    ] {
        let count = terms.len();

        let instr = AppendTermbaseInstr { terms };

        let val =
            append((&mock, &mock), token(), original_base.id.clone(), instr)
                .await
                .unwrap();

        assert_eq!((val.added, val.updated, val.unchanged), (0, 0, count));

        let snapshot = mock.snapshot();

        assert_eq!(
            snapshot.termbases.first().unwrap().updated_at,
            original_base.updated_at
        );

        assert_eq!(
            snapshot.terms.first().unwrap().updated_at,
            original_term.updated_at
        );
    }
}

#[tokio::test]
async fn append_rejects_invalid_batches_before_any_write() {
    let mock = Mock::new();

    seed_scope(&mock);

    let original_base = termbase(0);

    mock.seed_termbase(original_base.clone());

    for invalid in [
        append_term(" valid ", &["Duplicate source"], None),
        append_term(" ", &["Target"], None),
        append_term("Invalid", &[], None),
        append_term("Invalid", &[" "], None),
        append_term("Invalid", &["Same", " same "], None),
    ] {
        let instr = AppendTermbaseInstr {
            terms: vec![append_term("Valid", &["Target"], None), invalid],
        };

        let error =
            append((&mock, &mock), token(), original_base.id.clone(), instr)
                .await
                .unwrap_err();

        assert_expected_variant(&error, ExpectedVariant::Args);

        assert!(mock.snapshot().terms.is_empty());

        assert_eq!(
            mock.snapshot().termbases.first().unwrap().updated_at,
            original_base.updated_at
        );
    }
}

#[tokio::test]
async fn append_requires_existing_base_and_write_membership_even_when_empty() {
    let mock = Mock::new();

    seed_scope(&mock);

    let error = append(
        (&mock, &mock),
        token(),
        "missing".into(),
        AppendTermbaseInstr { terms: vec![] },
    )
    .await
    .unwrap_err();

    assert_expected_variant(&error, ExpectedVariant::Args);

    mock.seed_termbase(termbase(0));

    let mut reader = member("reader");

    reader.roles = RoleMask::from(RoleField::TYPESETTER);

    mock.seed_member(reader);

    for user_id in ["reader", "outsider"] {
        let token = UserToken {
            user_id: user_id.into(),
        };

        let error = append(
            (&mock, &mock),
            token,
            "termbase-1".into(),
            AppendTermbaseInstr { terms: vec![] },
        )
        .await
        .unwrap_err();

        assert_expected_variant(&error, ExpectedVariant::Perm);
    }
}

#[tokio::test]
async fn append_capacity_failure_keeps_existing_content() {
    let mock = Mock::new();

    seed_scope(&mock);

    let limit = crate::value::termbase::TERMBASE_TERM_LIMIT;

    mock.seed_termbase(termbase(limit));

    for index in 0..limit {
        mock.seed_term(term(
            &format!("term-{index}"),
            &format!("Source-{index}"),
            &["Old"],
        ));
    }

    let instr = AppendTermbaseInstr {
        terms: vec![
            append_term("Source-0", &["New"], Some("New note")),
            append_term("Added", &["Target"], None),
        ],
    };

    let error = append((&mock, &mock), token(), "termbase-1".into(), instr)
        .await
        .unwrap_err();

    assert_expected_variant(&error, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.terms.len(), limit);

    assert!(snapshot.terms.iter().all(|term| term.targets == ["Old"]
        && term.comment.as_deref() == Some("Old comment")));
}
