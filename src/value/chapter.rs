//! Chapter workflow stages, phases, and transition rules.

use std::result::Result;

use serde::{Deserialize, Serialize, Serializer};

use poprako_util::i18n::trl;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::incl::InclOpt;

// Keep chapter-specific tests colocated with the value-level invariants they verify.
#[cfg(test)]
mod tests;

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
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-invalid-stage-phase"),
        });
    }

    let next_phase = match (stage, phase, oper) {
        //
        (Stage::Publish, _, StageOper::Revert) => {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-invalid-workflow-transition"),
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
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-invalid-workflow-transition"),
            });
        }

        (_, StagePhase::Completed, StageOper::Revert) => StagePhase::Active,

        (_, StagePhase::Active, StageOper::Revert) => StagePhase::Pending,

        (_, StagePhase::Pending, StageOper::Revert) => StagePhase::Pending,
    };

    if !is_valid_stage_phase(stage, next_phase) {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
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

    // Convert one compact field into concrete phase value when known.
    //
    // Returns `None` for wildcard (`IGNORE`) so callers can explicitly decide
    // whether wildcard matching is allowed in this context.
    fn as_phase(self) -> Option<StagePhase> {
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
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-invalid-stage-phase"),
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
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-invalid-stage-phase"),
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
        Self::STAGES.iter().all(|stage| {
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
        stages
            .iter()
            .any(|s| self.get_phase(*s) != StagePhase::Pending)
    }

    /// Check if all of the given stages have a non-`Pending` phase.
    pub fn has_every_stage(&self, stages: &[Stage]) -> bool {
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
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-invalid-stage"),
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

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-invalid-stage-phase"),
            });
        }

        let Some(phase) = field.as_phase() else {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-invalid-stage-phase"),
            });
        };

        if !is_valid_stage_phase(stage, phase) {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-invalid-stage-phase"),
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

/// Incl opts for chapter info queries.
///
/// Each opt embeds additional related data into the returned
/// `ChapterInfoVal`. Dotted opts implicitly pull in the segments before the
/// dot (e.g. `comic.workset.team` also embeds `comic` and `comic.workset`).
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub enum ChapterInclOpt {
    //
    /// Embed the parent comic (`comic`).
    #[serde(rename = "comic")]
    Comic,

    /// Embed the comic and its workset (`comic.workset`; implies `comic`).
    #[serde(rename = "comic.workset")]
    ComicWorkset,

    /// Embed the comic, its workset, and the workset's team
    /// (`comic.workset.team`; implies `comic` and `comic.workset`).
    #[serde(rename = "comic.workset.team")]
    ComicWorksetTeam,

    /// Embed the comic and the comic's creating user
    /// (`comic.creator`; implies `comic`).
    #[serde(rename = "comic.creator")]
    ComicCreator,

    /// Embed the chapter's creating user (`creator`).
    #[serde(rename = "creator")]
    Creator,
}

impl InclOpt for ChapterInclOpt {
    // Return all include paths implied by the selected chapter inclusion option.
    fn path(self) -> &'static [Self] {
        match self {
            //
            Self::Comic => &[Self::Comic],

            Self::ComicWorkset => &[Self::Comic, Self::ComicWorkset],

            Self::ComicWorksetTeam => {
                &[Self::Comic, Self::ComicWorkset, Self::ComicWorksetTeam]
            }

            Self::ComicCreator => &[Self::Comic, Self::ComicCreator],

            Self::Creator => &[Self::Creator],
        }
    }
}
