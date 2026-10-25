#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// build_term_upsert_plan(build_term_upsert_plan)(positive): Unicode source matching preserves target order and spelling.
// build_term_upsert_plan(build_term_upsert_plan)(positive): duplicate existing keys retain the first match.
// build_term_upsert_plan(build_term_upsert_plan)(positive): a full terminology base accepts matching imports in import order.
// build_term_upsert_plan(build_term_upsert_plan)(negative): capacity and cached-count validation remain mandatory, including empty imports.

use time::OffsetDateTime;

use super::*;

// Build persisted terminology with deterministic timestamps.
fn term_info(id: &str, source: &str, targets: &[&str]) -> TermInfo {
    TermInfo {
        id: id.into(),
        termbase_id: "termbase--00000000001".into(),
        source: source.into(),
        targets: targets.iter().map(|target| (*target).into()).collect(),
        comment: None,
        creator_id: "original-creator".into(),
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    }
}

// Build an import through the production normalization boundary.
fn term_import<'a>(source: &'a str, targets: &[&'a str]) -> TermImport {
    term_complex::build_import(
        source.into(),
        targets.iter().map(|target| (*target).into()).collect(),
        Some("new comment".into()),
    )
    .unwrap()
}

#[test]
fn upsert_matches_unicode_sources_and_preserves_target_order() {
    //
    let existing = [
        term_info(
            "term--00000000001",
            "\u{2003}ÄPFEL\u{00a0}",
            &["ÄLT", "Second"],
        ),
        term_info("term--00000000002", "ΟΣ", &["Greek"]),
        term_info("term--00000000003", "İ", &["Turkish"]),
    ];

    let imported = vec![
        term_import(" äpfel ", &["ält", "New"]),
        term_import("Added", &["Added target"]),
        term_import("ος", &["Greek new"]),
        term_import("i\u{307}", &["Turkish new"]),
    ];

    let plan = build_term_upsert_plan(
        "termbase--00000000001",
        "creator-1",
        existing.len(),
        &existing,
        &imported,
    )
    .unwrap();

    assert_eq!(plan.entries.len(), 1);

    let entry = plan.entries.first().unwrap();

    assert_eq!(entry.source, "Added");

    assert!(
        crate::value::business_id::BusinessIdKind::Term.is_valid_id(&entry.id)
    );

    assert_eq!(entry.creator_id, "creator-1");

    assert_eq!(plan.updates.len(), 3);

    let update = plan.updates.first().unwrap();

    assert_eq!(update.id, "term--00000000001");

    assert_eq!(update.source, "äpfel");

    assert_eq!(update.targets, ["ÄLT", "Second", "New"]);

    assert_eq!(update.comment, Some("new comment"));

    assert_eq!(update.id.as_ptr(), existing.first().unwrap().id.as_ptr());

    assert_eq!(
        update.source.as_ptr(),
        imported.first().unwrap().trimmed_source().as_ptr()
    );

    assert_eq!(
        update.targets.first().unwrap().as_ptr(),
        existing.first().unwrap().targets.first().unwrap().as_ptr()
    );

    assert_eq!(
        update.targets.last().unwrap().as_ptr(),
        imported.first().unwrap().targets.last().unwrap().as_ptr()
    );

    assert_eq!(
        entry.source.as_ptr(),
        imported.get(1).unwrap().source.as_ptr()
    );

    assert_eq!(plan.updates.get(1).unwrap().id, "term--00000000002");

    assert_eq!(plan.updates.get(2).unwrap().id, "term--00000000003");
}

#[test]
fn upsert_preserves_first_existing_match_for_duplicate_keys() {
    //
    let existing = [
        term_info("first", "Source", &["First"]),
        term_info("second", " source ", &["Second"]),
    ];

    let incoming = vec![term_import("SOURCE", &["New"])];

    let plan = build_term_upsert_plan(
        "termbase--00000000001",
        "creator-1",
        existing.len(),
        &existing,
        &incoming,
    )
    .unwrap();

    let update = plan.updates.first().unwrap();

    assert_eq!(update.id, "first");

    assert_eq!(update.targets, ["First", "New"]);
}

#[test]
fn upsert_matches_a_full_termbase_in_import_order() {
    //
    let existing = (0..TERMBASE_TERM_LIMIT)
        .map(|index| {
            term_info(
                &format!("term-{index}"),
                &format!("Source-{index}"),
                &["Old"],
            )
        })
        .collect::<Vec<_>>();

    let sources = (0..TERMBASE_TERM_LIMIT)
        .rev()
        .map(|index| format!(" source-{index} "))
        .collect::<Vec<_>>();

    let imported = sources
        .iter()
        .map(|source| term_import(source, &["old", "New"]))
        .collect::<Vec<_>>();

    let incoming = vec![term_import("Added", &["New"])];

    let plan = build_term_upsert_plan(
        "termbase--00000000001",
        "creator-1",
        existing.len(),
        &existing,
        &imported,
    )
    .unwrap();

    assert!(plan.entries.is_empty());

    assert_eq!(plan.updates.len(), TERMBASE_TERM_LIMIT);

    for (update, expected_index) in
        plan.updates.iter().zip((0..TERMBASE_TERM_LIMIT).rev())
    {
        //
        assert_eq!(update.id, format!("term-{expected_index}"));

        assert_eq!(update.targets, ["Old", "New"]);
    }

    let error = build_term_upsert_plan(
        "termbase--00000000001",
        "creator-1",
        existing.len(),
        &existing,
        &incoming,
    )
    .err()
    .unwrap();

    assert!(matches!(
        error,
        BaseError::Expected {
            variant: ExpectedVariant::Args,
            ..
        }
    ));
}

#[test]
fn empty_import_preserves_count_validation() {
    //
    let plan = build_term_upsert_plan(
        "termbase--00000000001",
        "creator-1",
        0,
        &[],
        &[],
    )
    .unwrap();

    assert!(plan.entries.is_empty());

    assert!(plan.updates.is_empty());

    assert!(matches!(
        build_term_upsert_plan(
            "termbase--00000000001",
            "creator-1",
            1,
            &[],
            &[]
        ),
        Err(BaseError::Unrecoverable { .. }),
    ));
}

// append_comments(merge_append_comment)(positive): comments are whole blocks, never line or substring sets.
#[test]
fn append_comments_compare_complete_blocks() {
    for (existing, incoming, expected) in [
        (None, None, None),
        (Some("old"), None, Some("old")),
        (None, Some("new"), Some("new")),
        (Some("same"), Some("same"), Some("same")),
        (
            Some("first\nsecond"),
            Some("second"),
            Some("first\nsecond\nsecond"),
        ),
        (
            Some("first"),
            Some("first\nsecond"),
            Some("first\nfirst\nsecond"),
        ),
    ] {
        assert_eq!(
            merge_append_comment(existing, incoming).as_deref(),
            expected
        );
    }
}

// append_plan(build_term_append_plan)(positive): a full base accepts updates and retains Unicode source spelling.
#[test]
fn append_plan_preserves_unicode_spelling_and_accepts_full_base_updates() {
    let existing = (0..TERMBASE_TERM_LIMIT)
        .map(|index| {
            term_info(
                &format!("term-{index}"),
                &format!("ÄPFEL-{index}"),
                &["ÄLT", "Second"],
            )
        })
        .collect::<Vec<_>>();

    let incoming = vec![term_import("äpfel-0", &["ält", "New"])];

    let plan = build_term_append_plan(
        "termbase--00000000001",
        "creator",
        existing.len(),
        &existing,
        &incoming,
    )
    .unwrap();

    assert!(plan.entries.is_empty());

    let update = plan.updates.first().unwrap();

    assert_eq!(update.source, "ÄPFEL-0");

    assert_eq!(update.targets, ["ÄLT", "Second", "New"]);
}
