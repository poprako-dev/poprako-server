//! Value types for comic aggregates — incl opts for list queries.

use serde::Deserialize;

use crate::value::incl::InclOpt;

/// Incl opts for comic info queries.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ComicInclOpt {
    #[serde(rename = "workset")]
    Workset,

    #[serde(rename = "workset.team")]
    WorksetTeam,

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
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ComicWithOpt {
    PinnedChapter,
}
