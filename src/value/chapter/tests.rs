// validates_one_shot_phases(is_valid_stage_phase)(positive): one-shot stages accept pending and completed phases.
// validates_three_phase_phases(is_valid_stage_phase)(positive): three-phase stages accept all phases.
// rejects_active_one_shot_phase(is_valid_stage_phase)(negative): one-shot stages reject active phase.
// advances_one_shot_stage(try_modify_stage)(positive): one-shot stages advance from pending to completed.
// advances_three_phase_stage(try_modify_stage)(positive): three-phase stages advance through active to completed.
// reverts_three_phase_stage(try_modify_stage)(positive): three-phase stages revert through active to pending.
// accepts_pending_revert_noop(try_modify_stage)(positive): pending revert remains pending.
// rejects_publish_revert(try_modify_stage)(negative): publish cannot be reverted.
// rejects_completed_advance(try_modify_stage)(negative): completed stages cannot advance further.

use super::*;

#[test]
fn validates_one_shot_phases() {
    assert!(is_valid_stage_phase(
        WorkflowStage::RawProvide,
        StagePhase::Pending
    ));
    assert!(is_valid_stage_phase(
        WorkflowStage::Review,
        StagePhase::Completed
    ));
    assert!(is_valid_stage_phase(
        WorkflowStage::Publish,
        StagePhase::Completed
    ));
}

#[test]
fn validates_three_phase_phases() {
    for stage in [
        WorkflowStage::Translate,
        WorkflowStage::Proofread,
        WorkflowStage::TypesetRedraw,
    ] {
        assert!(is_valid_stage_phase(stage, StagePhase::Pending));
        assert!(is_valid_stage_phase(stage, StagePhase::Active));
        assert!(is_valid_stage_phase(stage, StagePhase::Completed));
    }
}

#[test]
fn rejects_active_one_shot_phase() {
    assert!(!is_valid_stage_phase(
        WorkflowStage::RawProvide,
        StagePhase::Active
    ));
    assert!(!is_valid_stage_phase(
        WorkflowStage::Review,
        StagePhase::Active
    ));
    assert!(!is_valid_stage_phase(
        WorkflowStage::Publish,
        StagePhase::Active
    ));
}

#[test]
fn advances_one_shot_stage() {
    let phase = try_modify_stage(
        (WorkflowStage::RawProvide, StagePhase::Pending),
        WorkflowEvent::Advance,
    )
    .ok()
    .unwrap();

    assert_eq!(phase, StagePhase::Completed);
}

#[test]
fn advances_three_phase_stage() {
    let phase = try_modify_stage(
        (WorkflowStage::Translate, StagePhase::Pending),
        WorkflowEvent::Advance,
    )
    .ok()
    .unwrap();
    assert_eq!(phase, StagePhase::Active);

    let phase = try_modify_stage((WorkflowStage::Translate, phase), WorkflowEvent::Advance)
        .ok()
        .unwrap();
    assert_eq!(phase, StagePhase::Completed);
}

#[test]
fn reverts_three_phase_stage() {
    let phase = try_modify_stage(
        (WorkflowStage::Proofread, StagePhase::Completed),
        WorkflowEvent::Revert,
    )
    .ok()
    .unwrap();
    assert_eq!(phase, StagePhase::Active);

    let phase = try_modify_stage((WorkflowStage::Proofread, phase), WorkflowEvent::Revert)
        .ok()
        .unwrap();
    assert_eq!(phase, StagePhase::Pending);
}

#[test]
fn accepts_pending_revert_noop() {
    let phase = try_modify_stage(
        (WorkflowStage::TypesetRedraw, StagePhase::Pending),
        WorkflowEvent::Revert,
    )
    .ok()
    .unwrap();

    assert_eq!(phase, StagePhase::Pending);
}

#[test]
fn rejects_publish_revert() {
    let err = try_modify_stage(
        (WorkflowStage::Publish, StagePhase::Completed),
        WorkflowEvent::Revert,
    )
    .err();

    assert!(err.is_some());
}

#[test]
fn rejects_completed_advance() {
    let err = try_modify_stage(
        (WorkflowStage::Translate, StagePhase::Completed),
        WorkflowEvent::Advance,
    )
    .err();

    assert!(err.is_some());
}
