use super::*;

use serde_json::json;

// update_chapter_stage_instr_uses_snake_case_stage(UpdateChapterStageInstr)(positive): JSON body stage names deserialize from snake_case.
#[test]
fn update_chapter_stage_instr_uses_snake_case_stage() {
    let instr = serde_json::from_value::<UpdateChapterStageInstr>(json!({
        "id": "chapter-1",
        "stage": "raw_provide",
        "oper": "advance",
    }))
    .unwrap();

    assert!(matches!(instr.stage, ChapterStageInstr::RawProvide));
}

// update_chapter_stage_instr_rejects_kebab_case_stage(UpdateChapterStageInstr)(negative): kebab-case is not accepted by a JSON body DTO.
#[test]
fn update_chapter_stage_instr_rejects_kebab_case_stage() {
    let result = serde_json::from_value::<UpdateChapterStageInstr>(json!({
        "id": "chapter-1",
        "stage": "raw-provide",
        "oper": "advance",
    }));

    assert!(result.is_err());
}
