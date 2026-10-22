// normalize_fuzzy_source(normalize_fuzzy_source)(positive): Unicode trimming borrows the original source and omits blank filters.
// build_entry(build_entry)(positive): normalized write fields borrow input buffers without modifying them.

#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use super::*;

#[test]
fn entry_borrows_normalized_source_targets_and_comment() {
    //
    let source = "\u{2003}Source\u{00a0}".to_string();

    let target = "\u{3000}Target\u{2009}".to_string();

    let comment = "\nComment\t".to_string();

    let entry = build_entry(
        "termbase-1".into(),
        &source,
        vec![&target],
        Some(&comment),
        "creator-1".into(),
    )
    .unwrap();

    assert_eq!(entry.source, "Source");

    assert_eq!(entry.source.as_ptr(), source.trim().as_ptr());

    assert_eq!(entry.targets, ["Target"]);

    assert_eq!(
        entry.targets.first().unwrap().as_ptr(),
        target.trim().as_ptr()
    );

    assert_eq!(entry.comment, Some("Comment"));

    assert_eq!(
        entry.comment.map(str::as_ptr),
        Some(comment.trim().as_ptr())
    );

    assert_eq!(source, "\u{2003}Source\u{00a0}");

    assert_eq!(target, "\u{3000}Target\u{2009}");

    assert_eq!(comment, "\nComment\t");
}

#[test]
fn fuzzy_source_borrows_trimmed_content_and_omits_blank_filters() {
    //
    let source = "\u{2003}Source\u{00a0}".to_string();

    let normalized = normalize_fuzzy_source(Some(&source));

    assert_eq!(normalized, Some("Source"));

    assert_eq!(normalized.map(str::as_ptr), Some(source.trim().as_ptr()));

    for source in [None, Some(""), Some(" \u{2003}\t\n")] {
        assert_eq!(normalize_fuzzy_source(source), None);
    }
}
