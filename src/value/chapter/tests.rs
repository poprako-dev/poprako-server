// validates_one_shot_phases(is_valid_stage_phase)(positive): one-shot stages accept pending and completed phases.
// validates_three_phase_phases(is_valid_stage_phase)(positive): three-phase stages accept all phases.
// rejects_active_one_shot_phase(is_valid_stage_phase)(negative): one-shot stages reject active phase.
// advances_one_shot_stage(try_modify_stage)(positive): one-shot stages advance from pending to completed.
// advances_three_phase_stage(try_modify_stage)(positive): three-phase stages advance through active to completed.
// reverts_three_phase_stage(try_modify_stage)(positive): three-phase stages revert through active to pending.
// accepts_pending_revert_noop(try_modify_stage)(positive): pending revert remains pending.
// rejects_publish_revert(try_modify_stage)(negative): publish cannot be reverted.
// rejects_completed_advance(try_modify_stage)(negative): completed stages cannot advance further.
// rejects_active_one_shot_mask(WorkflowStageMask::try_from)(negative): regular masks reject active one-shot stages.
// rejects_ignore_regular_mask(WorkflowStageMask::try_from)(negative): regular masks reject ignore fields.
// accepts_ignore_filter_mask(WorkflowStageMask::try_filter_from)(positive): filter masks accept ignore fields.
// rejects_active_one_shot_filter_mask(WorkflowStageMask::try_filter_from)(negative): filter masks reject active one-shot stages.
// rejects_invalid_set_phase(WorkflowStageMask::try_set_phase)(negative): setting an active one-shot phase is rejected.

use super::mask::StageMask;
use super::stage::{
    Stage, StageOper, StagePhase, is_valid_stage_phase, try_modify_stage,
};

#[test]
fn validates_one_shot_phases() {
    //
    assert!(is_valid_stage_phase(Stage::RawProvide, StagePhase::Pending));

    assert!(is_valid_stage_phase(Stage::Review, StagePhase::Completed));

    assert!(is_valid_stage_phase(Stage::Publish, StagePhase::Completed));
}

#[test]
fn validates_three_phase_phases() {
    for stage in [Stage::Translate, Stage::Proofread, Stage::TypesetRedraw] {
        //
        assert!(is_valid_stage_phase(stage, StagePhase::Pending));

        assert!(is_valid_stage_phase(stage, StagePhase::Active));

        assert!(is_valid_stage_phase(stage, StagePhase::Completed));
    }
}

#[test]
fn rejects_active_one_shot_phase() {
    //
    assert!(!is_valid_stage_phase(Stage::RawProvide, StagePhase::Active));

    assert!(!is_valid_stage_phase(Stage::Review, StagePhase::Active));

    assert!(!is_valid_stage_phase(Stage::Publish, StagePhase::Active));
}

#[test]
fn advances_one_shot_stage() {
    //
    let phase = try_modify_stage(
        (Stage::RawProvide, StagePhase::Pending),
        StageOper::Advance,
    )
    .ok()
    .unwrap();

    assert_eq!(phase, StagePhase::Completed);
}

#[test]
fn advances_three_phase_stage() {
    //
    let phase = try_modify_stage(
        (Stage::Translate, StagePhase::Pending),
        StageOper::Advance,
    )
    .ok()
    .unwrap();

    assert_eq!(phase, StagePhase::Active);

    let phase = try_modify_stage((Stage::Translate, phase), StageOper::Advance)
        .ok()
        .unwrap();

    assert_eq!(phase, StagePhase::Completed);
}

#[test]
fn reverts_three_phase_stage() {
    //
    let phase = try_modify_stage(
        (Stage::Proofread, StagePhase::Completed),
        StageOper::Revert,
    )
    .ok()
    .unwrap();

    assert_eq!(phase, StagePhase::Active);

    let phase = try_modify_stage((Stage::Proofread, phase), StageOper::Revert)
        .ok()
        .unwrap();

    assert_eq!(phase, StagePhase::Pending);
}

#[test]
fn accepts_pending_revert_noop() {
    //
    let phase = try_modify_stage(
        (Stage::TypesetRedraw, StagePhase::Pending),
        StageOper::Revert,
    )
    .ok()
    .unwrap();

    assert_eq!(phase, StagePhase::Pending);
}

#[test]
fn rejects_publish_revert() {
    //
    let err = try_modify_stage(
        (Stage::Publish, StagePhase::Completed),
        StageOper::Revert,
    )
    .err();

    assert!(err.is_some());
}

#[test]
fn rejects_completed_advance() {
    //
    let err = try_modify_stage(
        (Stage::Translate, StagePhase::Completed),
        StageOper::Advance,
    )
    .err();

    assert!(err.is_some());
}

#[test]
fn rejects_active_one_shot_mask() {
    //
    let active_raw_provide_mask = 0b01;

    let err = StageMask::try_from(active_raw_provide_mask).err();

    assert!(err.is_some());
}

#[test]
fn rejects_ignore_regular_mask() {
    //
    let ignore_translate_mask = 0b11 << 2;

    let err = StageMask::try_from(ignore_translate_mask).err();

    assert!(err.is_some());
}

#[test]
fn accepts_ignore_filter_mask() {
    //
    let ignore_translate_mask = 0b11 << 2;

    let mask = StageMask::try_filter_from(ignore_translate_mask)
        .ok()
        .unwrap();

    assert!(mask.ignores_stage(Stage::Translate));
}

#[test]
fn rejects_active_one_shot_filter_mask() {
    //
    let active_review_mask = 0b01 << 8;

    let err = StageMask::try_filter_from(active_review_mask).err();

    assert!(err.is_some());
}

#[test]
fn rejects_invalid_set_phase() {
    //
    let mask = StageMask::try_from(0u32).ok().unwrap();

    let err = mask.try_set_phase(Stage::Publish, StagePhase::Active).err();

    assert!(err.is_some());
}
