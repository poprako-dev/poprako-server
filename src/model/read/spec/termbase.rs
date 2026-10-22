//! Domain models for team- and comic-scoped terminology bases.

use crate::value::pagination::PubListLimit;

/// Filtering and pagination parameters for terminology-base lists.
pub enum TermbaseListSpec<'a> {
    /// List terminology bases directly owned by a team.
    Team {
        /// ID of the team whose termbases to list.
        team_id: &'a str,
        /// Optional fuzzy name filter.
        fuzzy_name: Option<&'a str>,
        /// Number of records to skip for pagination.
        offset: u32,
        /// Maximum number of records to return.
        limit: PubListLimit,
    },

    /// List terminology bases visible from a comic.
    Comic {
        /// ID of the comic whose associated termbases to list.
        comic_id: &'a str,
        /// Optional fuzzy name filter.
        fuzzy_name: Option<&'a str>,
        /// Number of records to skip for pagination.
        offset: u32,
        /// Maximum number of records to return.
        limit: PubListLimit,
    },
}
