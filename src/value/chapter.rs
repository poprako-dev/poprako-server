//! Chapter workflow stages, phases, and transition rules.

use serde::{Deserialize, Serialize};

use poprako_util::i18n::trl;

use crate::result::{ExpectedVariant, RootError, RootResult, accept};

/// Phase a workflow stage can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StagePhase {
    Pending,
    Active,
    Completed,
}

/// Stage in the chapter production pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowStage {
    /// 上传
    RawProvide,
    /// 翻译
    Translate,
    /// 校对
    Proofread,
    /// 嵌字/修图
    TypesetRedraw,
    /// 监修
    Review,
    /// 发布
    Publish,
}

/// Validate that a [`StagePhase`] is legal for the given [`WorkflowStage`].
///
/// `RawProvide`, `Review`, and `Publish` cannot be `Active` (they are
/// instantaneous stages). `Translate`, `Proofread`, and `TypesetRedraw`
/// accept any phase.
pub fn is_valid_stage_phase(stage: WorkflowStage, phase: StagePhase) -> bool {
    match stage {
        WorkflowStage::RawProvide | WorkflowStage::Review | WorkflowStage::Publish => {
            matches!(phase, StagePhase::Pending | StagePhase::Completed)
        }
        WorkflowStage::Translate | WorkflowStage::Proofread | WorkflowStage::TypesetRedraw => true,
    }
}

/// Event that triggers a workflow stage transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowEvent {
    /// Advance to the next phase.
    Advance,
    /// Revert to the previous phase.
    Revert,
}

/// Apply a [`WorkflowEvent`] to a stage and return the resulting [`StagePhase`],
/// or error if the transition is illegal.
pub fn try_modify_stage(
    current: (WorkflowStage, StagePhase),
    event: WorkflowEvent,
) -> RootResult<StagePhase> {
    let (stage, phase) = current;

    if !is_valid_stage_phase(stage, phase) {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-invalid-stage-phase"),
        });
    }

    let next_phase = match (stage, phase, event) {
        (WorkflowStage::Publish, _, WorkflowEvent::Revert) => {
            return Err(RootError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-invalid-workflow-transition"),
            });
        }
        (
            WorkflowStage::RawProvide | WorkflowStage::Review | WorkflowStage::Publish,
            StagePhase::Pending,
            WorkflowEvent::Advance,
        ) => StagePhase::Completed,
        (
            WorkflowStage::RawProvide | WorkflowStage::Review,
            StagePhase::Completed,
            WorkflowEvent::Revert,
        ) => StagePhase::Pending,
        (_, StagePhase::Pending, WorkflowEvent::Advance) => StagePhase::Active,
        (_, StagePhase::Active, WorkflowEvent::Advance) => StagePhase::Completed,
        (_, StagePhase::Completed, WorkflowEvent::Advance) => {
            return Err(RootError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-invalid-workflow-transition"),
            });
        }
        (_, StagePhase::Completed, WorkflowEvent::Revert) => StagePhase::Active,
        (_, StagePhase::Active, WorkflowEvent::Revert) => StagePhase::Pending,
        (_, StagePhase::Pending, WorkflowEvent::Revert) => StagePhase::Pending,
    };

    if !is_valid_stage_phase(stage, next_phase) {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-invalid-stage-phase"),
        });
    }

    accept(next_phase)
}

#[cfg(test)]
mod tests {
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
}
