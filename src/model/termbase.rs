/// A termbase record: named glossary scoped to a team or a comic.
pub struct TermbaseInfo {
    pub id: String,

    pub name: String,

    pub team_id: Option<String>,
    pub comic_id: Option<String>,
}

/// A termbase entry variant used during creation — scoped to exactly one of
/// a team or a comic.
pub enum TermbaseEntry {
    /// Termbase owned by a team.
    Team {
        id: String,
        name: String,
        team_id: String,
    },
    /// Termbase owned by a comic.
    Comic {
        id: String,
        name: String,
        comic_id: String,
    },
}
