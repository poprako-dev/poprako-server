use super::*;

#[test]
fn owned_url_moves_its_text_and_serializes_as_a_string() {
    //
    let text = String::from("https://obj.test/cover?signature=owned");

    let pointer = text.as_ptr();

    let view = ObjUrlView::from(text);

    assert_eq!(view.as_ptr(), pointer);

    assert!(matches!(&view.text, ObjUrlText::Owned { .. }));

    assert_eq!(
        serde_json::to_value(&view).unwrap(),
        "https://obj.test/cover?signature=owned",
    );

    let optional = Some(view);

    assert_eq!(
        optional.as_deref(),
        Some("https://obj.test/cover?signature=owned"),
    );
}

#[test]
fn shared_url_moves_its_handle_and_preserves_string_serialization() {
    //
    let text = Arc::new(String::from("https://obj.test/avatar"));

    let pointer = text.as_ptr();

    let first = ObjUrlView::from(Arc::clone(&text));

    let second = ObjUrlView::from(text);

    assert_eq!(first.as_ptr(), pointer);

    assert_eq!(second.as_ptr(), pointer);

    let ObjUrlText::Shared { text: shared } = &second.text else {
        panic!("repeated URL must retain shared text");
    };

    assert_eq!(Arc::strong_count(shared), 2);

    assert_eq!(
        serde_json::to_value([first, second]).unwrap(),
        serde_json::json!([
            "https://obj.test/avatar",
            "https://obj.test/avatar",
        ]),
    );
}
