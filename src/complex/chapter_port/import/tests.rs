// parse_label_plus(parse_label_plus)(positive): parses the real LabelPlus material.
// parse_poprako(parse_poprako)(positive): normalizes PopRaKo one-based unit indexes.

use super::*;

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
