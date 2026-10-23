// normalize_fuzzy_source(normalize_fuzzy_source)(positive): Unicode trimming borrows the original source and omits blank filters.

#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use super::*;

// build_entry(build_entry)(positive): input buffers move into the model and normalized accessors borrow them.
#[test]
fn entry_owns_input_and_borrows_normalized_fields() {
    let source = "\u{2003}Source\u{00a0}".to_string();

    let target = "\u{3000}Target\u{2009}".to_string();

    let comment = "\nComment\t".to_string();

    let source_ptr = source.as_ptr();

    let target_ptr = target.as_ptr();

    let comment_ptr = comment.as_ptr();

    let entry = build_entry(
        "termbase-1".into(),
        source,
        vec![target],
        Some(comment),
        "creator-1".into(),
    )
    .unwrap();

    assert_eq!(entry.source.as_ptr(), source_ptr);

    assert_eq!(entry.targets.first().unwrap().as_ptr(), target_ptr);

    assert_eq!(entry.comment.as_ref().unwrap().as_ptr(), comment_ptr);

    assert_eq!(entry.trimmed_source(), "Source");

    assert_eq!(entry.trimmed_targets().collect::<Vec<_>>(), ["Target"]);

    assert_eq!(entry.trimmed_comment(), Some("Comment"));

    assert_eq!(
        entry.trimmed_source().as_ptr(),
        entry.source.trim().as_ptr()
    );
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
