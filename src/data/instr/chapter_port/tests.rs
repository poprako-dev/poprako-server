use super::*;

use serde_json::json;

// import_chapter_translation_instr_uses_snake_case_format(ImportChapterTranslationInstr)(positive): JSON body formats deserialize from snake_case.
#[test]
fn import_chapter_translation_instr_uses_snake_case_format() {
    let instr =
        serde_json::from_value::<ImportChapterTranslationInstr>(json!({
            "format": "label_plus",
            "mode": "keep",
            "content": "content",
        }))
        .unwrap();

    assert!(matches!(
        instr.format,
        ChapterTranslationFormatInstr::LabelPlus
    ));

    assert!(matches!(
        instr.mode,
        ChapterTranslationImportModeInstr::Keep
    ));
}

// import_chapter_translation_instr_rejects_kebab_case_format(ImportChapterTranslationInstr)(negative): kebab-case is not accepted by a JSON body DTO.
#[test]
fn import_chapter_translation_instr_rejects_kebab_case_format() {
    let result =
        serde_json::from_value::<ImportChapterTranslationInstr>(json!({
            "format": "label-plus",
            "mode": "overwrite",
            "content": "content",
        }));

    assert!(result.is_err());
}

// import_chapter_translation_instr_requires_mode(ImportChapterTranslationInstr)(negative): import callers must explicitly choose whether existing page content is preserved or replaced.
#[test]
fn import_chapter_translation_instr_requires_mode() {
    let result =
        serde_json::from_value::<ImportChapterTranslationInstr>(json!({
            "format": "label_plus",
            "content": "content",
        }));

    assert!(result.is_err());
}

// import_chapter_translation_instr_rejects_invalid_mode(ImportChapterTranslationInstr)(negative): only the documented snake_case import modes are accepted.
#[test]
fn import_chapter_translation_instr_rejects_invalid_mode() {
    let result =
        serde_json::from_value::<ImportChapterTranslationInstr>(json!({
            "format": "label_plus",
            "mode": "replace",
            "content": "content",
        }));

    assert!(result.is_err());
}
