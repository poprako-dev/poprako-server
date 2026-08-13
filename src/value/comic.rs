//! Value types for comic aggregates — incl opts for list queries.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::value::incl::InclOpt;

/// Lifecycle state used to filter comics in management lists.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ComicStatus {
    /// A comic with active child resources.
    Active,

    /// A comic retained only as an immutable archive header.
    Archived,
}

/// Incl opts for comic info queries.
///
/// Each opt embeds additional related data into the returned
/// [`ComicInfoView`]. Dotted opts implicitly pull in the segments before the
/// dot (e.g. `workset.team` also embeds `workset`).
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub enum ComicInclOpt {
    //
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
    // Expand comic include options into required dependency path segments.
    fn path(self) -> &'static [Self] {
        //
        match self {
            //
            Self::Workset => &[Self::Workset],

            Self::WorksetTeam => &[Self::Workset, Self::WorksetTeam],

            Self::Creator => &[Self::Creator],
        }
    }
}

/// Extra data options for comic info queries.
///
/// Unlike `ComicInclOpt`, `with` opts populate the separately returned
/// derived data (e.g. the chapter currently pinned to each comic).
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ComicWithOpt {
    //
    /// Populate the parallel pinned-chapter list.
    PinnedChapter,

    /// Populate assignments for each pinned chapter. Requires
    /// `pinned_chapter` in the same request.
    PinnedChapterAssignment,
}
