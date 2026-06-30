//! Value types for comic aggregates — include options for list queries.

use serde::Deserialize;

/// Include options for comic info queries.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ComicInclOpt {
    Workset,
    Team,
    Creator,
}

/// Extra data options for comic info queries.
pub enum ComicWithOpt {
    PinnedChapter,
}
