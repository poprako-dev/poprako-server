//! Chapter workflow stages, phases, and transition rules.

use serde::{Deserialize, Serialize, Serializer};

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
    /// Raw provide phase.
    RawProvide,
    /// Translate phase.
    Translate,
    /// Proofread phase.
    Proofread,
    /// Typeset and redraw phase.
    TypesetRedraw,
    /// Review phase.
    Review,
    /// Publish phase.
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
            variant: ExpectedVariant::ArgsInvalid,
            message: trl("error-invalid-stage-phase"),
        });
    }

    let next_phase = match (stage, phase, event) {
        (WorkflowStage::Publish, _, WorkflowEvent::Revert) => {
            return Err(RootError::Expected {
                variant: ExpectedVariant::ArgsInvalid,
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
                variant: ExpectedVariant::ArgsInvalid,
                message: trl("error-invalid-workflow-transition"),
            });
        }
        (_, StagePhase::Completed, WorkflowEvent::Revert) => StagePhase::Active,
        (_, StagePhase::Active, WorkflowEvent::Revert) => StagePhase::Pending,
        (_, StagePhase::Pending, WorkflowEvent::Revert) => StagePhase::Pending,
    };

    if !is_valid_stage_phase(stage, next_phase) {
        return Err(RootError::Expected {
            variant: ExpectedVariant::ArgsInvalid,
            message: trl("error-invalid-stage-phase"),
        });
    }

    accept(next_phase)
}

/// A singular stage phase value (pending, active, completed, or ignore).
///
/// Unlike a bitmask, these values are mutually exclusive discriminants
/// (not bit positions). `IGNORE` is a wildcard matching any phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StagePhaseField(u8);

impl StagePhaseField {
    pub const PENDING: Self = Self(0);
    pub const ACTIVE: Self = Self(1);
    pub const COMPLETED: Self = Self(2);
    pub const IGNORE: Self = Self(3);

    const VALID_VALUES: &'static [u8] = &[0, 1, 2, 3];
}

impl TryFrom<u8> for StagePhaseField {
    type Error = RootError;

    fn try_from(value: u8) -> RootResult<Self> {
        if !Self::VALID_VALUES.contains(&value) {
            return Err(RootError::Expected {
                variant: ExpectedVariant::ArgsInvalid,
                message: trl("error-invalid-stage-phase"),
            });
        }

        accept(Self(value))
    }
}

impl From<StagePhaseField> for u8 {
    fn from(value: StagePhaseField) -> Self {
        value.0
    }
}

/// Convert a [`StagePhase`] (business enum) into its storage field.
impl From<StagePhase> for StagePhaseField {
    fn from(phase: StagePhase) -> Self {
        match phase {
            StagePhase::Pending => Self::PENDING,
            StagePhase::Active => Self::ACTIVE,
            StagePhase::Completed => Self::COMPLETED,
        }
    }
}

impl Serialize for StagePhaseField {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(u8::from(*self))
    }
}

/// A composite bitmask storing the phase of all 6 workflow stages.
///
/// Each stage occupies 2 bits (4 possible states matching
/// [`StagePhaseField`]), ordered from low bits:
///
/// | Stage | Bits | Field |
/// |:---:|:---:|:---:|
/// | RawProvide | 0–1 | `StagePhaseField` |
/// | Translate | 2–3 | `StagePhaseField` |
/// | Proofread | 4–5 | `StagePhaseField` |
/// | TypesetRedraw | 6–7 | `StagePhaseField` |
/// | Review | 8–9 | `StagePhaseField` |
/// | Publish | 10–11 | `StagePhaseField` |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowStageMask(u32);

impl WorkflowStageMask {
    const VALID_BITS: u32 = (1 << 12) - 1;

    fn stage_shift(stage: WorkflowStage) -> u32 {
        match stage {
            WorkflowStage::RawProvide => 0,
            WorkflowStage::Translate => 2,
            WorkflowStage::Proofread => 4,
            WorkflowStage::TypesetRedraw => 6,
            WorkflowStage::Review => 8,
            WorkflowStage::Publish => 10,
        }
    }

    /// Extract the [`StagePhase`] for a specific stage.
    pub fn get_phase(&self, stage: WorkflowStage) -> StagePhase {
        match (self.0 >> Self::stage_shift(stage)) as u8 & 0b11 {
            0 => StagePhase::Pending,
            1 => StagePhase::Active,
            2 => StagePhase::Completed,
            _ => unreachable!("stored phase is always 0-2"),
        }
    }

    /// Return a new mask with the given stage's phase set.
    pub fn set_phase(&self, stage: WorkflowStage, phase: StagePhase) -> Self {
        let shift = Self::stage_shift(stage);

        Self(self.0 & !(0b11 << shift) | ((u8::from(StagePhaseField::from(phase)) as u32) << shift))
    }

    /// Check if a specific stage has the given phase.
    pub fn has_phase(&self, stage: WorkflowStage, phase: StagePhase) -> bool {
        self.get_phase(stage) == phase
    }

    /// Check if any of the given stages has a non-`Pending` phase.
    pub fn has_any_stage(&self, stages: &[WorkflowStage]) -> bool {
        stages
            .iter()
            .any(|s| self.get_phase(*s) != StagePhase::Pending)
    }

    /// Check if all of the given stages have a non-`Pending` phase.
    pub fn has_every_stage(&self, stages: &[WorkflowStage]) -> bool {
        stages
            .iter()
            .all(|s| self.get_phase(*s) != StagePhase::Pending)
    }

    /// Check if the mask fully contains another mask's phases.
    ///
    /// For each 2-bit slot, `self`'s bits must be a superset of `other`'s
    /// bits (i.e. a `PENDING` slot in `other` is always contained; an
    /// `IGNORE` slot in `self` contains any phase in `other`).
    pub fn contains_mask(&self, other: WorkflowStageMask) -> bool {
        self.0 & other.0 == other.0
    }

    /// Return the union of two masks (bitwise OR per 2-bit slot).
    pub fn union(&self, other: WorkflowStageMask) -> WorkflowStageMask {
        Self(self.0 | other.0)
    }
}

impl TryFrom<u32> for WorkflowStageMask {
    type Error = RootError;

    fn try_from(value: u32) -> RootResult<Self> {
        if value & !Self::VALID_BITS != 0 {
            return Err(RootError::Expected {
                variant: ExpectedVariant::ArgsInvalid,
                message: trl("error-invalid-stage"),
            });
        }

        accept(Self(value))
    }
}

impl From<WorkflowStageMask> for u32 {
    fn from(value: WorkflowStageMask) -> Self {
        value.0
    }
}

impl Serialize for WorkflowStageMask {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.0)
    }
}

/// Include options for chapter info queries.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ChapterInclOpt {
    Creator,
}

#[cfg(test)]
mod tests;
