use super::*;

use serde_json::json;

// import_chapter_translation_instr_uses_snake_case_format(ImportChapterTranslationInstr)(positive): JSON body formats deserialize from snake_case.
#[test]
fn import_chapter_translation_instr_uses_snake_case_format() {
    let instr =
        serde_json::from_value::<ImportChapterTranslationInstr>(json!({
            "format": "label_plus",
            "content": "content",
        }))
        .unwrap();

    assert!(matches!(
        instr.format,
        ChapterTranslationFormatInstr::LabelPlus
    ));
}

// import_chapter_translation_instr_rejects_kebab_case_format(ImportChapterTranslationInstr)(negative): kebab-case is not accepted by a JSON body DTO.
#[test]
fn import_chapter_translation_instr_rejects_kebab_case_format() {
    let result =
        serde_json::from_value::<ImportChapterTranslationInstr>(json!({
            "format": "label-plus",
            "content": "content",
        }));

    assert!(result.is_err());
}
