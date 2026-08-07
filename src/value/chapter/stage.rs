use serde::{Deserialize, Serialize, Serializer};

use poprako_util::i18n::trl;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Phase a workflow stage can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "kebab-case")]
pub enum StagePhase {
    //
    /// The stage has not started yet.
    Pending,

    /// The stage is actively being worked on.
    Active,

    /// The stage has been completed.
    Completed,
}

/// Stage in the chapter production pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "kebab-case")]
pub enum Stage {
    //
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
pub fn is_valid_stage_phase(stage: Stage, phase: StagePhase) -> bool {
    //
    match stage {
        //
        Stage::RawProvide | Stage::Review | Stage::Publish => {
            matches!(phase, StagePhase::Pending | StagePhase::Completed)
        }

        Stage::Translate | Stage::Proofread | Stage::TypesetRedraw => true,
    }
}

/// Operation applied to a workflow stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "kebab-case")]
pub enum StageOper {
    //
    /// Advance to the next phase.
    Advance,

    /// Revert to the previous phase.
    Revert,
}

/// Apply a [`StageOper`] to a stage and return the resulting [`StagePhase`],
/// or error if the transition is illegal.
pub fn try_modify_stage(
    current: (Stage, StagePhase),
    oper: StageOper,
) -> BaseRest<StagePhase> {
    //
    let (stage, phase) = current;

    if !is_valid_stage_phase(stage, phase) {
        //
        let err_message = trl("error-invalid-stage-phase");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            stage = ?stage,
            phase = ?phase,
            oper = ?oper,
            "expected error: invalid current stage phase",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    let next_phase = match (stage, phase, oper) {
        //
        (Stage::Publish, _, StageOper::Revert) => {
            //
            let err_message = trl("error-invalid-workflow-transition");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                stage = ?stage,
                phase = ?phase,
                oper = ?oper,
                "expected error: invalid workflow transition",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        (
            Stage::RawProvide | Stage::Review | Stage::Publish,
            StagePhase::Pending,
            StageOper::Advance,
        ) => StagePhase::Completed,

        (
            Stage::RawProvide | Stage::Review,
            StagePhase::Completed,
            StageOper::Revert,
        ) => StagePhase::Pending,

        (_, StagePhase::Pending, StageOper::Advance) => StagePhase::Active,

        (_, StagePhase::Active, StageOper::Advance) => StagePhase::Completed,

        (_, StagePhase::Completed, StageOper::Advance) => {
            //
            let err_message = trl("error-invalid-workflow-transition");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                stage = ?stage,
                phase = ?phase,
                oper = ?oper,
                "expected error: invalid workflow transition",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        (_, StagePhase::Completed, StageOper::Revert) => StagePhase::Active,

        (_, StagePhase::Active, StageOper::Revert) => StagePhase::Pending,

        (_, StagePhase::Pending, StageOper::Revert) => StagePhase::Pending,
    };

    if !is_valid_stage_phase(stage, next_phase) {
        //
        let err_message = trl("error-invalid-stage-phase");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            stage = ?stage,
            phase = ?phase,
            next_phase = ?next_phase,
            oper = ?oper,
            "expected error: invalid next stage phase",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
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
    // Accepted serialized values for phase fields.
    /// Field value representing [`StagePhase::Pending`].
    pub const PENDING: Self = Self(0);
    /// Field value representing [`StagePhase::Active`].
    pub const ACTIVE: Self = Self(1);
    /// Field value representing [`StagePhase::Completed`].
    pub const COMPLETED: Self = Self(2);
    /// Wildcard value that matches any phase — used in filter masks.
    pub const IGNORE: Self = Self(3);

    // Shared lookup table for allowed raw numeric phase values.
    const VALID_VALUES: &'static [u8] = &[0, 1, 2, 3];

    /// Convert one compact field into concrete phase value when known.
    ///
    /// Returns `None` for wildcard (`IGNORE`) so callers can explicitly decide
    /// whether wildcard matching is allowed in this context.
    pub fn as_phase(self) -> Option<StagePhase> {
        //
        match self {
            //
            Self::PENDING => Some(StagePhase::Pending),

            Self::ACTIVE => Some(StagePhase::Active),

            Self::COMPLETED => Some(StagePhase::Completed),

            Self::IGNORE => None,

            _ => unreachable!("stage phase field is validated at construction"),
        }
    }
}

// Convert a [`StagePhase`] (business enum) into its storage field.
impl From<StagePhase> for StagePhaseField {
    // Map domain phase into corresponding compact field representation.
    fn from(phase: StagePhase) -> Self {
        //
        match phase {
            //
            StagePhase::Pending => Self::PENDING,

            StagePhase::Active => Self::ACTIVE,

            StagePhase::Completed => Self::COMPLETED,
        }
    }
}

impl TryFrom<u8> for StagePhaseField {
    // Error returned when a stored phase field code is out of range.
    type Error = BaseError;

    // Validate and construct a stage phase field from raw DB/API code.
    fn try_from(value: u8) -> BaseRest<Self> {
        //
        if !Self::VALID_VALUES.contains(&value) {
            //
            let err_message = trl("error-invalid-stage-phase");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                raw_value = value,
                "expected error: invalid stage phase field",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        accept(Self(value))
    }
}

impl Serialize for StagePhaseField {
    // Serialize phase field as a single unsigned byte value.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(u8::from(*self))
    }
}

impl From<StagePhaseField> for u8 {
    // Return the wire/storage value used by compact bitmask encoding.
    fn from(value: StagePhaseField) -> Self {
        value.0
    }
}
