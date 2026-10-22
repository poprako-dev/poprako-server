use super::subtitle_or_default;

use std::borrow::Cow;

#[test]
fn subtitle_or_default_trims_unicode_whitespace() {
    assert_eq!(
        subtitle_or_default(Some("\u{2003}Chapter\u{00a0}"), 0),
        "Chapter"
    );
}

#[test]
fn subtitle_or_default_uses_default_after_trimming_empty_input() {
    assert_eq!(
        subtitle_or_default(Some("\u{2003}\t\u{00a0}"), 0),
        subtitle_or_default(None, 0)
    );
}

// subtitle_or_default(subtitle_or_default)(positive): request text stays borrowed while generated defaults own their text.
#[test]
fn subtitle_or_default_owns_only_generated_defaults() {
    //
    let input = "\u{2003}Chapter\u{00a0}".to_string();

    let subtitle = subtitle_or_default(Some(&input), 0);

    assert!(matches!(subtitle, Cow::Borrowed("Chapter")));

    assert_eq!(subtitle.as_ptr(), input.trim().as_ptr());

    assert!(matches!(subtitle_or_default(None, 0), Cow::Owned(_)));

    assert_eq!(input, "\u{2003}Chapter\u{00a0}");
}
