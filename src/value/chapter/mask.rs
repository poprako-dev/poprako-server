use serde::{Serialize, Serializer};

use poprako_util::i18n::trl;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::chapter::stage::{
    Stage, StagePhase, StagePhaseField, is_valid_stage_phase,
};

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
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct StageMask(u32);

impl StageMask {
    // High bits outside the known six-stage, two-bit layout are invalid.
    const VALID_BITS: u32 = (1 << 12) - 1;

    // Canonical stage order used for iteration, validation and matching.
    const STAGES: &'static [Stage] = &[
        Stage::RawProvide,
        Stage::Translate,
        Stage::Proofread,
        Stage::TypesetRedraw,
        Stage::Review,
        Stage::Publish,
    ];

    /// Construct a filter mask from raw bits.
    ///
    /// Filter masks may use `IGNORE` fields as wildcards, but they still
    /// reject stage-phase combinations that are impossible for real workflow.
    pub fn try_filter_from(value: u32) -> BaseRest<Self> {
        //
        Self::validate_mask_value(value, true)?;

        accept(Self(value))
    }

    /// Return workflow stages in mask bit order.
    pub fn stages() -> &'static [Stage] {
        Self::STAGES
    }

    /// Extract the [`StagePhase`] for a specific stage.
    pub fn get_phase(&self, stage: Stage) -> StagePhase {
        //
        self.field_for_stage(stage)
            .as_phase()
            .expect("regular workflow masks never contain ignore fields")
    }

    /// Return a new mask with the given stage's phase set.
    pub fn try_set_phase(
        &self,
        stage: Stage,
        phase: StagePhase,
    ) -> BaseRest<Self> {
        //
        if !is_valid_stage_phase(stage, phase) {
            //
            let err_message = trl("error-invalid-stage-phase");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                stage = ?stage,
                phase = ?phase,
                current_mask = self.0,
                "expected error: invalid stage phase for mask",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        let shift = Self::stage_shift(stage);

        let value = self.0 & !(0b11 << shift)
            | ((u8::from(StagePhaseField::from(phase)) as u32) << shift);

        Self::try_from(value)
    }

    /// Check if a filter mask ignores a specific stage.
    pub fn ignores_stage(&self, stage: Stage) -> bool {
        self.field_for_stage(stage) == StagePhaseField::IGNORE
    }

    /// Check if this regular mask satisfies a filter mask.
    pub fn matches_filter(&self, filter: StageMask) -> bool {
        //
        Self::STAGES.iter().all(|stage| {
            //
            filter.ignores_stage(*stage)
                || self.field_for_stage(*stage)
                    == filter.field_for_stage(*stage)
        })
    }

    /// Check if a specific stage has the given phase.
    pub fn has_phase(&self, stage: Stage, phase: StagePhase) -> bool {
        self.get_phase(stage) == phase
    }

    /// Check if any of the given stages has a non-`Pending` phase.
    pub fn has_any_stage(&self, stages: &[Stage]) -> bool {
        //
        stages
            .iter()
            .any(|s| self.get_phase(*s) != StagePhase::Pending)
    }

    /// Check if all of the given stages have a non-`Pending` phase.
    pub fn has_every_stage(&self, stages: &[Stage]) -> bool {
        //
        stages
            .iter()
            .all(|s| self.get_phase(*s) != StagePhase::Pending)
    }

    /// Check if the mask fully contains another mask's phases.
    ///
    /// For each 2-bit slot, `self`'s bits must be a superset of `other`'s
    /// bits (i.e. a `PENDING` slot in `other` is always contained; an
    /// `IGNORE` slot in `self` contains any phase in `other`).
    pub fn contains_mask(&self, other: StageMask) -> bool {
        self.0 & other.0 == other.0
    }

    /// Return the union of two masks (bitwise OR per 2-bit slot).
    pub fn union(&self, other: StageMask) -> StageMask {
        Self(self.0 | other.0)
    }

    // Validate an entire packed mask, including reserved bits and per-stage limits.
    fn validate_mask_value(value: u32, allow_ignore: bool) -> BaseRest<()> {
        //
        if value & !Self::VALID_BITS != 0 {
            //
            let err_message = trl("error-invalid-stage");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                raw_value = value,
                allow_ignore,
                "expected error: invalid stage mask bits",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        for stage in Self::STAGES {
            //
            let field = Self::field_for_stage_value(value, *stage)?;

            Self::validate_stage_field(*stage, field, allow_ignore)?;
        }

        accept(())
    }

    // Decode the requested stage field from `self`.
    fn field_for_stage(&self, stage: Stage) -> StagePhaseField {
        Self::field_for_stage_value(self.0, stage).ok().unwrap()
    }

    // Return the bit offset (in two-bit slots) for the given stage.
    fn stage_shift(stage: Stage) -> u32 {
        //
        match stage {
            //
            Stage::RawProvide => 0,

            Stage::Translate => 2,

            Stage::Proofread => 4,

            Stage::TypesetRedraw => 6,

            Stage::Review => 8,

            Stage::Publish => 10,
        }
    }

    // Decode one stage field from packed mask bits.
    fn field_for_stage_value(
        value: u32,
        stage: Stage,
    ) -> BaseRest<StagePhaseField> {
        //
        StagePhaseField::try_from(
            ((value >> Self::stage_shift(stage)) & 0b11) as u8,
        )
    }

    // Validate stage field rules for one stage.
    //
    // `allow_ignore` controls whether wildcard values are valid in this check.
    fn validate_stage_field(
        stage: Stage,
        field: StagePhaseField,
        allow_ignore: bool,
    ) -> BaseRest<()> {
        //
        if field == StagePhaseField::IGNORE {
            //
            if allow_ignore {
                return accept(());
            }

            let err_message = trl("error-invalid-stage-phase");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                stage = ?stage,
                field = ?field,
                allow_ignore,
                "expected error: ignored stage phase is not allowed",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        let Some(phase) = field.as_phase() else {
            //
            let err_message = trl("error-invalid-stage-phase");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                stage = ?stage,
                field = ?field,
                allow_ignore,
                "expected error: invalid stage phase field",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        };

        if !is_valid_stage_phase(stage, phase) {
            //
            let err_message = trl("error-invalid-stage-phase");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                stage = ?stage,
                phase = ?phase,
                field = ?field,
                allow_ignore,
                "expected error: invalid stage phase combination",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        accept(())
    }
}

impl TryFrom<u32> for StageMask {
    // Error returned when a packed mask cannot be interpreted as valid workflow state.
    type Error = BaseError;

    // Validate packed bits and build a concrete mask value.
    fn try_from(value: u32) -> BaseRest<Self> {
        //
        Self::validate_mask_value(value, false)?;

        accept(Self(value))
    }
}

impl Serialize for StageMask {
    // Serialize mask as an integer with one 2-bit slice per stage.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.0)
    }
}

impl From<StageMask> for u32 {
    // Convert mask back into compact numeric form for DB/persistence.
    fn from(value: StageMask) -> Self {
        value.0
    }
}
