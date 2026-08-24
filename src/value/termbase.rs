//! Terminology-base scope and capacity values.

/// Maximum number of terminology entries stored in one terminology base.
pub const TERMBASE_TERM_LIMIT: i32 = 200;

/// Target ownership scope selected for a terminology-base import.
pub enum TermbaseScope {
    //
    /// Import into a team-owned terminology base.
    Team {
        /// Team owning the selected terminology base.
        team_id: String,
    },

    /// Import into a comic-owned terminology base.
    Comic {
        /// Comic owning the selected terminology base.
        comic_id: String,
    },
}
