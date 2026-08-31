use super::*;

fn chapter_subtitle(language: &LanguageIdentifier) -> String {
    let mut args = HashMap::new();

    args.insert(
        Cow::Borrowed("number"),
        FluentValue::String(Cow::Borrowed("1")),
    );

    LOCALES.lookup_with_args(language, "chapter-default-subtitle", &args)
}

#[test]
fn parameterized_messages_have_exact_bytes_without_fluent_isolates() {
    let zh_subtitle = chapter_subtitle(&langid!("zh-CN"));

    let en_subtitle = chapter_subtitle(&langid!("en-US"));

    assert_eq!(zh_subtitle, "第1话");

    assert_eq!(
        zh_subtitle.as_bytes(),
        &[0xe7, 0xac, 0xac, 0x31, 0xe8, 0xaf, 0x9d]
    );

    assert_eq!(en_subtitle, "Ch. 1");

    assert_eq!(en_subtitle.as_bytes(), b"Ch. 1");
}
