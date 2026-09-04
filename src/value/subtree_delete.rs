//! Hierarchy tombstone sweep values.

/// One hierarchy level eligible for tombstone sweeping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubtreeSweepLevel {
    //
    /// Sweep one Chapter.
    Chapter,

    /// Sweep a batch of Comics.
    Comic,

    /// Sweep a batch of Worksets.
    Workset,

    /// Sweep one Team.
    Team,
}
