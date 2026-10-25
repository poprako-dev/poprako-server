//! Compatibility between exported response views and native import documents.

use crate::complex::chapter_port::import_translation as chapter_translation_import_complex;
use crate::data::view::chapter_port::ChapterTranslationPortView;
use crate::data::view::page_source_port::PageSourceTranslationPortView;
use crate::data::view::unit_port::UnitTranslationPortView;
use crate::model::shared::translation_document::ChapterTranslationDocument;
use crate::model::write::translation_import::UnitTranslationImportSource;

// Build an unordered native export with independent annotations.
fn document() -> ChapterTranslationPortView {
    ChapterTranslationPortView {
        chapter_id: "source-chapter".into(),
        chapter_index: 4,
        chapter_subtitle: Some("source subtitle".into()),
        comic_id: "source-comic".into(),
        comic_title: "source title".into(),
        pages: vec![
            PageSourceTranslationPortView {
                page_source_id: "source-page-1".into(),
                page_source_index: 1,
                units: vec![UnitTranslationPortView {
                    unit_id: "source-unit-1".into(),
                    unit_index: 1,
                    page_source_id: "source-page-1".into(),
                    page_source_index: 1,
                    x_coord: 0.4,
                    y_coord: 0.5,
                    is_bubble: false,
                    is_flagged: false,
                    typography_note: Some("小字\n\n括号".into()),
                    translation_note: Some("存疑".into()),
                    translated_text: Some("second".into()),
                    translator_id: Some("source-translator".into()),
                    is_proofread: false,
                    proofread_text: None,
                    proofreader_id: None,
                }],
            },
            PageSourceTranslationPortView {
                page_source_id: "source-page-0".into(),
                page_source_index: 0,
                units: vec![
                    UnitTranslationPortView {
                        unit_id: "source-unit-1".into(),
                        unit_index: 1,
                        page_source_id: "source-page-0".into(),
                        page_source_index: 0,
                        x_coord: 0.2,
                        y_coord: 0.3,
                        is_bubble: true,
                        is_flagged: false,
                        typography_note: None,
                        translation_note: None,
                        translated_text: Some("first".into()),
                        translator_id: None,
                        is_proofread: false,
                        proofread_text: None,
                        proofreader_id: None,
                    },
                    UnitTranslationPortView {
                        unit_id: "source-unit-2".into(),
                        unit_index: 0,
                        page_source_id: "source-page-0".into(),
                        page_source_index: 0,
                        x_coord: 0.1,
                        y_coord: 0.2,
                        is_bubble: true,
                        is_flagged: false,
                        typography_note: None,
                        translation_note: None,
                        translated_text: None,
                        translator_id: None,
                        is_proofread: false,
                        proofread_text: None,
                        proofreader_id: None,
                    },
                ],
            },
        ],
    }
}

#[test]
fn export_view_roundtrips_native_import_and_sorts_indexes() {
    let document = document();

    let original_json = serde_json::to_value(&document).unwrap();

    let native_document = serde_json::from_value::<ChapterTranslationDocument>(
        original_json.clone(),
    )
    .unwrap();

    let exported_view = ChapterTranslationPortView::from(native_document);

    assert_eq!(serde_json::to_value(&exported_view).unwrap(), original_json);

    let content = serde_json::to_string(&exported_view).unwrap();

    let pages =
        chapter_translation_import_complex::parse_poprako(&content).unwrap();

    assert_eq!(pages.first().unwrap().page_source_index, 0);
    assert_eq!(pages.first().unwrap().units.first().unwrap().index, 0);
    assert_eq!(pages.first().unwrap().units.get(1).unwrap().index, 1);
    assert_eq!(pages.get(1).unwrap().page_source_index, 1);

    assert_eq!(
        pages
            .get(1)
            .unwrap()
            .units
            .first()
            .unwrap()
            .typography_note
            .as_deref(),
        Some("小字\n\n括号")
    );

    assert_eq!(
        pages
            .get(1)
            .unwrap()
            .units
            .first()
            .unwrap()
            .translation_note
            .as_deref(),
        Some("存疑")
    );
    assert!(matches!(
        &pages.get(1).unwrap().units.first().unwrap().source,
        UnitTranslationImportSource::PopRaKo {
            translated_text: Some(text),
            ..
        } if text == "second"
    ));
}
