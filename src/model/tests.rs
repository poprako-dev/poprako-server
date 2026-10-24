use crate::model::shared::unit::UnitTranslation;
use crate::model::write::term::TermRepl;
use crate::model::write::termbase::TermbaseRepl;
use crate::model::write::workset::WorksetRepl;

// trimmed_description(trimmed_description)(positive): profile presence is retained while terminology blank values become absent.
#[test]
fn optional_normalization_preserves_each_fields_presence_contract() {
    for (raw, profile, terminology) in [
        (None, None, None),
        (Some(""), Some(""), None),
        (Some(" \u{2003}\n"), Some(""), None),
        (Some(" \u{2003}Text\u{00a0}"), Some("Text"), Some("Text")),
    ] {
        let workset = WorksetRepl {
            id: "workset".into(),
            name: "Name".into(),
            description: raw.map(str::to_owned),
        };

        let termbase = TermbaseRepl {
            id: "termbase".into(),
            name: "Name".into(),
            description: raw.map(str::to_owned),
        };

        let term = TermRepl {
            id: "term".into(),
            source: "Source".into(),
            targets: vec!["Target".into()],
            comment: raw.map(str::to_owned),
        };

        assert_eq!(workset.trimmed_description(), profile);

        assert_eq!(termbase.trimmed_description(), terminology);

        assert_eq!(term.trimmed_comment(), terminology);

        assert_eq!(workset.description.as_deref(), raw);

        assert_eq!(termbase.description.as_deref(), raw);

        assert_eq!(term.comment.as_deref(), raw);
    }
}

// trimmed_translated_text(trimmed_translated_text)(positive): predicates borrow a view while formatted text keeps its owned allocation.
#[test]
fn translation_view_preserves_multiline_content_and_buffer() {
    let raw = "\n  First line\n    Second line  \n".to_owned();

    let pointer = raw.as_ptr();

    let translation = UnitTranslation {
        translated_text: raw,
        last_translator_id: "translator".into(),
    };

    assert_eq!(translation.translated_text.as_ptr(), pointer);

    assert_eq!(
        translation.trimmed_translated_text(),
        "First line\n    Second line"
    );

    assert_eq!(
        translation.translated_text,
        "\n  First line\n    Second line  \n"
    );

    assert_eq!(
        translation.trimmed_translated_text().as_ptr(),
        translation.translated_text.trim().as_ptr()
    );
}
