pub struct TermbaseInfo {
    pub id: String,

    pub name: String,

    pub team_id: Option<String>,
    pub comic_id: Option<String>,
}

pub enum TermbaseEntry {
    Team {
        id: String,
        name: String,
        team_id: String,
    },
    Comic {
        id: String,
        name: String,
        comic_id: String,
    },
}
