// Integration fixture for real LabelPlus import parsing.
// parse_poprako(parse_poprako)(positive): normalizes PopRaKo one-based unit indexes.
// build_unit_create(build_unit_create)(positive): import always produces a complete Unit Create.

use super::*;

// LabelPlus import fixture used by chapter import parse tests.
const LABEL_PLUS_MATERIAL: &str =
    include_str!("../../../../tests/materials/translations.lp.txt");

#[test]
fn parse_label_plus_parses_real_material() {
    //
    let pages = ChapterImportComplex::parse_label_plus(LABEL_PLUS_MATERIAL);

    let pages = match pages {
        //
        Ok(pages) => pages,

        Err(_) => panic!("expected LabelPlus material parse success"),
    };

    assert_eq!(pages.len(), 9);

    assert_eq!(pages[0].units.len(), 10);

    assert_eq!(pages[8].units.len(), 9);

    assert_eq!(pages[0].units[0].index, 0);

    assert_eq!(pages[0].units[0].main_text, Some("喂 游斗哥".into()));

    assert_eq!(
        pages[8].units[8].main_text,
        Some("哥哥对次女可爱的\n小心思毫无察觉".into())
    );
}

#[test]
fn parse_poprako_normalizes_one_based_indexes() {
    //
    let pages = ChapterImportComplex::parse_poprako(
        r#"{
            "author": "author",
            "title": "title",
            "pages": [
                {
                    "image_filename": "001.png",
                    "units": [
                        {
                            "id": "unit-1",
                            "x": 0.1,
                            "y": 0.2,
                            "index_in_page": 7,
                            "is_inbox": true,
                            "translated_text": "translated",
                            "prooved_text": null,
                            "is_prooved": false
                        }
                    ]
                }
            ]
        }"#,
    );

    let pages = match pages {
        //
        Ok(pages) => pages,

        Err(_) => panic!("expected PopRaKo parse success"),
    };

    assert_eq!(pages[0].units[0].index, 6);
}

#[test]
fn build_unit_create_produces_a_complete_create() {
    //
    let pages =
        ChapterImportComplex::parse_label_plus(LABEL_PLUS_MATERIAL).unwrap();

    let edit = ChapterImportComplex::build_unit_create(
        &pages[0].units[0],
        "unit-new".to_string(),
        "proofreader-1",
        false,
        true,
        true,
    );

    assert!(matches!(
        edit,
        UnitEdit::Create {
            id,
            next_id: None,
            is_bubble: true,
            revision: Some(_),
            ..
        } if id == "unit-new"
    ));
}
