//! Chapter workflow stages, phases, and transition rules.

use serde::Deserialize;

use crate::value::incl::InclOpt;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

pub use mask::StageMask;
pub use stage::{
    Stage, StageOper, StagePhase, StagePhaseField, is_valid_stage_phase,
    try_modify_stage,
};

// Stage-phase bitmask helpers.
mod mask;
// Workflow stage, phase, and transition rules.
mod stage;

// Keep chapter-specific tests colocated with the value-level invariants they verify.
#[cfg(test)]
mod tests;

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

/// Incl opts for chapter info queries.
///
/// Each opt embeds additional related data into the returned
/// `ChapterInfoView`. Dotted opts implicitly pull in the segments before the
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
        //
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
