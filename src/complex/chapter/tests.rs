use super::subtitle_or_default;

#[test]
fn subtitle_or_default_preserves_owned_input() {
    assert_eq!(
        subtitle_or_default(Some("\u{2003}Chapter\u{00a0}".into()), 0),
        "\u{2003}Chapter\u{00a0}"
    );
}

#[test]
fn subtitle_or_default_uses_default_after_trimming_empty_input() {
    assert_eq!(
        subtitle_or_default(Some("\u{2003}\t\u{00a0}".into()), 0),
        subtitle_or_default(None, 0)
    );
}

// subtitle_or_default(subtitle_or_default)(positive): a supplied subtitle retains its allocation and defaults own their generated text.
#[test]
fn subtitle_or_default_moves_supplied_buffer() {
    let input = "\u{2003}Chapter\u{00a0}".to_string();

    let input_ptr = input.as_ptr();

    let subtitle = subtitle_or_default(Some(input), 0);

    assert_eq!(subtitle.as_ptr(), input_ptr);

    assert_eq!(subtitle.trim(), "Chapter");

    assert!(!subtitle_or_default(None, 0).is_empty());
}
