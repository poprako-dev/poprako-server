//! Chapter workflow stages, phases, and transition rules.

/// Stage-phase bitmask helpers.
pub mod mask;
/// Workflow stage, phase, and transition rules.
pub mod stage;

// Keep chapter-specific tests colocated with the value-level invariants they verify.
#[cfg(test)]
mod tests;

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::value::incl::InclOpt;

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
