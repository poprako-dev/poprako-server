//! Value types for comic aggregates — include options for list queries.

/// Include options for comic info queries.
pub enum ComicInclOpt {
    Workset,
    Team,
    Creator,
}

/// Extra data options for comic info queries.
pub enum ComicWithOpt {
    PinnedChapter,
}
