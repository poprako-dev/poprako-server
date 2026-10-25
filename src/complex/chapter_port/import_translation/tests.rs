#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use crate::complex::chapter_port::import_translation as chapter_translation_import_complex;
// Integration fixture for real LabelPlus import parsing.
// parse_poprako(parse_poprako)(positive): preserves zero-based PopRaKo indexes.
// build_unit_create(build_unit_create)(positive): import always produces a complete Unit Create.
// parse_label_plus(parse_label_plus)(positive): buffered text preserves meaningful whitespace and annotation syntax across units.

use super::*;

use crate::model::write::translation_import::UnitTranslationImportSource;

// LabelPlus import fixture used by chapter import parse tests.
const LABEL_PLUS_MATERIAL: &str =
    include_str!("../../../../tests/materials/translations.lp.txt");

#[test]
fn parse_label_plus_parses_real_material() {
    //
    let pages = chapter_translation_import_complex::parse_label_plus(
        LABEL_PLUS_MATERIAL,
    );

    let pages = pages.unwrap();

    assert_eq!(pages.len(), 9);

    assert_eq!(pages.first().unwrap().units.len(), 10);

    assert_eq!(pages.get(8).unwrap().units.len(), 9);

    assert_eq!(pages.first().unwrap().units.first().unwrap().index, 0);

    assert!(!pages.first().unwrap().units.first().unwrap().is_flagged);

    assert!(matches!(
        &pages.first().unwrap().units.first().unwrap().source,
        UnitTranslationImportSource::LabelPlus { text }
            if text.as_deref() == Some("喂 游斗哥")
    ));

    assert!(matches!(
        &pages.get(8).unwrap().units.get(8).unwrap().source,
        UnitTranslationImportSource::LabelPlus { text }
            if text.as_deref() == Some("哥哥对次女可爱的\n小心思毫无察觉")
    ));
}

#[test]
fn parse_poprako_preserves_zero_based_indexes() {
    //
    let pages = chapter_translation_import_complex::parse_poprako(
        r#"{
            "chapter_id": "chapter--00000000001",
            "chapter_index": 0,
            "chapter_subtitle": null,
            "comic_id": "comic--00000000001",
            "comic_title": "title",
            "pages": [
                {
                    "page_id": "page_source--00000000001",
                    "page_index": 0,
                    "units": [
                        {
                            "unit_id": "unit--00000000001",
                            "unit_index": 7,
                            "page_id": "page_source--00000000001",
                            "page_index": 0,
                            "x_coord": 0.1,
                            "y_coord": 0.2,
                            "is_bubble": true,
                            "translated_text": "translated",
                            "translator_id": null,
                            "is_proofread": false,
                            "proofread_text": null,
                            "proofreader_id": null
                        }
                    ]
                }
            ]
        }"#,
    );

    let pages = pages.unwrap();

    assert_eq!(pages.first().unwrap().units.first().unwrap().index, 7);

    assert!(!pages.first().unwrap().units.first().unwrap().is_flagged);

    assert!(matches!(
        &pages.first().unwrap().units.first().unwrap().source,
        UnitTranslationImportSource::PopRaKo {
            translated_text: Some(text),
            proofread_text: None,
            is_proofread: false,
        } if text == "translated"
    ));
}

#[test]
fn build_unit_create_produces_a_complete_create() {
    //
    let pages = chapter_translation_import_complex::parse_label_plus(
        LABEL_PLUS_MATERIAL,
    )
    .unwrap();

    let edit = chapter_translation_import_complex::build_unit_create(
        pages.first().unwrap().units.first().unwrap(),
        "unit--98124474837".to_string(),
        "proofreader-1",
        false,
        true,
    );

    assert!(matches!(
        edit,
        UnitEdit::Create {
            id,
            next_id: None,
            is_bubble: true,
            is_flagged: false,
            revision: Some(_),
            ..
        } if id == "unit--98124474837"
    ));
}

#[test]
fn parse_label_plus_accepts_bom_crlf_and_structure_trailing_whitespace() {
    let content = concat!(
        "\u{feff}1,0\r\n",
        "- \t\r\n",
        "框内\r\n",
        "框外\r\n",
        "-\t\r\n",
        "note\r\n",
        ">>>>>>>>[000.jpg]<<<<<<<< \t\r\n",
        "----------------[1]----------------[0.1,0.2,1] \t\r\n",
        " translated  \r\n",
    );

    let pages =
        chapter_translation_import_complex::parse_label_plus(content).unwrap();

    assert_eq!(pages.len(), 1);
    assert!(matches!(
        &pages.first().unwrap().units.first().unwrap().source,
        UnitTranslationImportSource::LabelPlus { text }
            if text.as_deref() == Some(" translated  ")
    ));
}

#[test]
fn parse_label_plus_preserves_text_and_annotations_across_buffer_reuse() {
    //
    let preamble = concat!(
        "1,0\n-\n框内\n框外\n-\nnote\n",
        ">>>>>>>>[000.jpg]<<<<<<<<\n",
        "----------------[1]----------------[0.1,0.2,1]\n",
    );

    let next_unit = "----------------[2]----------------[0.3,0.4,1]";

    for (body, expected_text, expected_note) in [
        ("", None, None),
        (" \u{2003}\t\n", None, None),
        ("\n\n body \n\n", Some(" body "), None),
        (
            "  first  \n\n second \n",
            Some("  first  \n\n second "),
            None,
        ),
        (
            "  body  \n\ninternal\n\n### note \nnext ###",
            Some("  body  \n\ninternal"),
            Some(" note \nnext "),
        ),
        ("### note ###", None, Some(" note ")),
        ("body\n\n### \u{2003} ###", Some("body"), None),
        ("body\n\n###note", Some("body\n\n###note"), None),
        (
            "body\n\n###nested###note###",
            Some("body\n\n###nested###note###"),
            None,
        ),
    ] {
        //
        let content = format!("{preamble}{body}\n{next_unit}\n next ");

        let pages =
            chapter_translation_import_complex::parse_label_plus(&content)
                .unwrap();

        let units = &pages.first().unwrap().units;

        assert_eq!(units.len(), 2);

        let unit = units.first().unwrap();

        assert!(
            matches!(
                &unit.source,
                UnitTranslationImportSource::LabelPlus { text }
                    if text.as_deref() == expected_text
            ),
            "body: {body:?}"
        );

        assert_eq!(
            unit.typography_note.as_deref(),
            expected_note,
            "body: {body:?}"
        );

        let next = units.get(1).unwrap();

        assert!(matches!(
            &next.source,
            UnitTranslationImportSource::LabelPlus { text }
                if text.as_deref() == Some(" next ")
        ));

        assert!(next.typography_note.is_none());
    }
}

#[test]
fn parse_poprako_rejects_duplicate_page_source_indexes() {
    let content = r#"{
        "chapter_id": "chapter--00000000001",
        "chapter_index": 0,
        "chapter_subtitle": null,
        "comic_id": "comic--00000000001",
        "comic_title": "title",
        "pages": [
            { "page_id": "page_source--00000000001", "page_index": 0, "units": [] },
            { "page_id": "page_source--00000000002", "page_index": 0, "units": [] }
        ]
    }"#;

    assert!(
        chapter_translation_import_complex::parse_poprako(content).is_err()
    );
}
