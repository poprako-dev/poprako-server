// make_label_plus(notes)(positive): annotations roundtrip independently of selected body and private communication.
// parse_label_plus(notes)(negative): ordinary and incomplete markers remain body text.

use super::*;

use crate::complex::chapter_port::import_translation as chapter_import_complex;
use crate::model::write::translation_import::UnitTranslationImportSource;

#[test]
fn label_plus_annotations_roundtrip_body_selection_and_blank_lines() {
    for (translation, revision, note, expected_body) in [
        (
            "译文",
            Some("校对\n\n第二段"),
            Some(" 小字\n\n括号 "),
            Some("校对\n\n第二段"),
        ),
        ("译文", Some(""), Some("小字"), Some("译文")),
        ("", None, Some("小字"), None),
        ("译文", None, None, Some("译文")),
        ("译文", None, Some(" \t\u{3000}"), Some("译文")),
        ("", None, None, None),
    ] {
        let mut unit_info =
            unit("unit-1", "page-1", None, translation, revision);

        unit_info.typography_note = note.map(str::to_owned);

        unit_info.translation_note = Some("private-discussion".into());

        let output = chapter_translation_export_complex::make_label_plus(
            &[page("page-1", 0, true)],
            &HashMap::from([("page-1".into(), vec![unit_info])]),
            &HashMap::new(),
            &HashMap::new(),
        );

        assert!(!output.contains("private-discussion"));

        let expected_note = note.filter(|value| !value.trim().is_empty());

        if let (Some(body), Some(note)) = (expected_body, expected_note) {
            assert!(output.contains(&format!("{body}\n\n###{note}###")));
        }

        assert_eq!(output.contains("###"), expected_note.is_some());

        let pages = chapter_import_complex::parse_label_plus(&output).unwrap();

        let imported = pages.first().unwrap().units.first().unwrap();

        assert_eq!(imported.typography_note.as_deref(), expected_note);

        assert!(imported.translation_note.is_none());

        assert!(
            matches!(&imported.source, UnitTranslationImportSource::LabelPlus { text }
            if text.as_deref() == expected_body)
        );
    }
}

#[test]
fn label_plus_only_extracts_a_complete_suffix_annotation() {
    for (text, body, note) in [
        ("正文 ###普通###", Some("正文 ###普通###"), None),
        ("###普通###\n后续正文", Some("###普通###\n后续正文"), None),
        ("正文\n\n###未闭合", Some("正文\n\n###未闭合"), None),
        ("正文\n\n尾部###", Some("正文\n\n尾部###"), None),
        (
            "正文\n\n###多行\n\n注释###",
            Some("正文"),
            Some("多行\n\n注释"),
        ),
        ("正文\n\n######", Some("正文"), None),
        ("###注释###", None, Some("注释")),
    ] {
        let content = format!(
            "1,0\n-\n框内\n框外\n-\nnotes\n>>>>>>>>[000.jpg]<<<<<<<<\n----------------[1]----------------[0.1,0.2,1]\n{text}\n\n"
        );

        for document in [content.clone(), content.replace('\n', "\r\n")] {
            let pages =
                chapter_import_complex::parse_label_plus(&document).unwrap();

            let imported = pages.first().unwrap().units.first().unwrap();

            assert_eq!(imported.typography_note.as_deref(), note);

            assert!(
                matches!(&imported.source, UnitTranslationImportSource::LabelPlus { text }
                if text.as_deref() == body)
            );
        }
    }
}

#[tokio::test]
async fn export_exposes_annotations_as_independent_json_fields() {
    let mock = Mock::new();

    seed_scope(&mock);

    let mut unit_info = unit("unit-1", "page-1", None, "正文", Some("校对"));

    unit_info.typography_note = Some("小字".into());

    unit_info.translation_note = Some("存疑".into());

    mock.seed_unit(unit_info);

    let exported = export_translation(
        (&mock, &mock, &mock),
        token("user-1"),
        "chapter-1".into(),
        ExportFormatSpec::BOTH,
        false,
    )
    .await
    .unwrap();

    let document = exported.poprako.as_ref().unwrap();

    let exported_unit = document.pages.first().unwrap().units.first().unwrap();

    assert_eq!(exported_unit.typography_note.as_deref(), Some("小字"));

    assert_eq!(exported_unit.translation_note.as_deref(), Some("存疑"));

    assert_eq!(exported_unit.translated_text.as_deref(), Some("正文"));

    assert_eq!(exported_unit.proofread_text.as_deref(), Some("校对"));

    assert!(exported.label_plus.unwrap().contains("校对\n\n###小字###"));
}
