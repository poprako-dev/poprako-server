use super::subtitle_or_default;

#[test]
fn subtitle_or_default_trims_unicode_whitespace() {
    assert_eq!(
        subtitle_or_default(Some("\u{2003}Chapter\u{00a0}".into()), 0),
        "Chapter"
    );
}

#[test]
fn subtitle_or_default_uses_default_after_trimming_empty_input() {
    assert_eq!(
        subtitle_or_default(Some("\u{2003}\t\u{00a0}".into()), 0),
        subtitle_or_default(None, 0)
    );
}
