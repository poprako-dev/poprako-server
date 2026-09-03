//! Minimal persisted projections for hierarchical mark-and-sweep deletion.

/// A locked hierarchy root with the ancestry needed for permission checks and
/// surviving-parent updates.
pub enum SubtreeDeleteScope {
    //
    /// A team root.
    Team {
        /// Team identifier.
        team_id: String,
    },

    /// A workset root.
    Workset {
        /// Workset identifier.
        workset_id: String,
        /// Owning team identifier.
        team_id: String,
    },

    /// A comic root.
    Comic {
        /// Comic identifier.
        comic_id: String,
        /// Parent workset identifier.
        workset_id: String,
        /// Owning team identifier.
        team_id: String,
    },

    /// A chapter root.
    Chapter {
        /// Chapter identifier.
        chapter_id: String,
        /// Parent comic identifier.
        comic_id: String,
        /// Parent workset identifier.
        workset_id: String,
        /// Owning team identifier.
        team_id: String,
        /// Whether the deleted chapter must trigger repinning.
        was_pinned: bool,
    },
}

impl SubtreeDeleteScope {
    /// Returns the team that owns this hierarchy root.
    #[must_use]
    pub const fn team_id(&self) -> &String {
        //
        match self {
            //
            Self::Team { team_id }
            | Self::Workset { team_id, .. }
            | Self::Comic { team_id, .. }
            | Self::Chapter { team_id, .. } => team_id,
        }
    }
}

/// One tombstoned hierarchy row claimed for physical cleanup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubtreeDeleteSweepTarget {
    //
    /// A chapter and all of its page-owned records.
    Chapter {
        /// Chapter identifier.
        id: String,
    },

    /// A comic whose physical chapter rows are already gone.
    Comic {
        /// Comic identifier.
        id: String,
    },

    /// A workset whose physical comic rows are already gone.
    Workset {
        /// Workset identifier.
        id: String,
    },

    /// A team whose physical workset rows are already gone.
    Team {
        /// Team identifier.
        id: String,
    },
}
