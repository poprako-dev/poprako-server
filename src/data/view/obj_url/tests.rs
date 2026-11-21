#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use super::*;

#[test]
fn owned_url_keeps_its_text_when_converted_into_a_string() {
    //
    let text = String::from("https://obj.test/cover?signature=owned");

    let pointer = text.as_ptr();

    let view = ObjUrlView::from(text);

    assert!(matches!(&view.text, ObjUrlText::Owned { .. }));

    let converted = view.into_string();

    assert_eq!(converted, "https://obj.test/cover?signature=owned");

    assert_eq!(converted.as_ptr(), pointer);
}

#[test]
fn shared_url_clones_while_shared_and_moves_its_handle_when_alone() {
    //
    let text = Arc::new(String::from("https://obj.test/avatar"));

    let pointer = text.as_ptr();

    let first = ObjUrlView::from(Arc::clone(&text));

    let second = ObjUrlView::from(text);

    assert_eq!(first.as_ptr(), pointer);

    assert_eq!(second.as_ptr(), pointer);

    assert_eq!(first.into_string(), "https://obj.test/avatar");

    let converted = second.into_string();

    assert_eq!(converted, "https://obj.test/avatar");

    assert_eq!(converted.as_ptr(), pointer);
}
