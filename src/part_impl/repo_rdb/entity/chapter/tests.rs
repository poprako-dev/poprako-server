// derives_workflow_mask_from_timestamps(ChapterInfo::try_from)(positive): timestamp columns derive legal workflow phases.

use super::*;

fn row() -> ChapterRow {
    let time = OffsetDateTime::now_utc();

    ChapterRow {
        f_id: "chapter-1".into(),
        f_comic_id: "comic-1".into(),
        f_is_pinned: true,
        f_index: 0,
        f_subtitle: "Chapter".into(),
        f_page_count: 0,
        f_total_unit_count: 0,
        f_translated_unit_count: 0,
        f_proofread_unit_count: 0,
        f_uploaded_at: Some(time),
        f_translating_at: Some(time),
        f_translated_at: Some(time),
        f_proofreading_at: Some(time),
        f_proofread_at: None,
        f_typesetting_at: None,
        f_typeset_at: None,
        f_reviewed_at: Some(time),
        f_published_at: None,
        f_creator_id: "user-1".into(),
        f_created_at: time,
        f_updated_at: time,
    }
}

#[test]
fn derives_workflow_mask_from_timestamps() {
    let chapter_info = ChapterInfo::try_from(row()).ok().unwrap();

    assert_eq!(
        chapter_info.stages.get_phase(WorkflowStage::RawProvide),
        StagePhase::Completed
    );

    assert_eq!(
        chapter_info.stages.get_phase(WorkflowStage::Translate),
        StagePhase::Completed
    );

    assert_eq!(
        chapter_info.stages.get_phase(WorkflowStage::Proofread),
        StagePhase::Active
    );

    assert_eq!(
        chapter_info.stages.get_phase(WorkflowStage::TypesetRedraw),
        StagePhase::Pending
    );

    assert_eq!(
        chapter_info.stages.get_phase(WorkflowStage::Review),
        StagePhase::Completed
    );

    assert_eq!(
        chapter_info.stages.get_phase(WorkflowStage::Publish),
        StagePhase::Pending
    );
}
