//! Value types for comic aggregates — incl opts for list queries.

use serde::Deserialize;

use utoipa::ToSchema;

use crate::value::incl::InclOpt;

/// Incl opts for comic info queries.
///
/// Each opt embeds additional related data into the returned
/// [`ComicInfoVal`]. Dotted opts implicitly pull in the segments before the
/// dot (e.g. `workset.team` also embeds `workset`).
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
pub enum ComicInclOpt {
    /// Embed the parent workset (`workset`).
    #[serde(rename = "workset")]
    Workset,

    /// Embed the workset and its owning team (`workset.team`; implies `workset`).
    #[serde(rename = "workset.team")]
    WorksetTeam,

    /// Embed the creating user (`creator`).
    #[serde(rename = "creator")]
    Creator,
}

impl InclOpt for ComicInclOpt {
    fn path(self) -> &'static [Self] {
        match self {
            Self::Workset => &[Self::Workset],
            Self::WorksetTeam => &[Self::Workset, Self::WorksetTeam],
            Self::Creator => &[Self::Creator],
        }
    }
}

/// Extra data options for comic info queries.
///
/// Unlike `ComicInclOpt`, `with` opts attach derived rather than
/// directly related data (e.g. the chapter currently pinned to the comic).
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ComicWithOpt {
    /// Embed the chapter currently pinned to each comic.
    PinnedChapter,
}
