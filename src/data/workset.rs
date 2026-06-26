//! Data transfer objects for workset use cases.

/// Presentation-ready workset information.
pub struct WorksetInfoVal {
    pub id: String,
    pub team_id: String,

    pub index: i32,

    pub name: String,
    pub description: Option<String>,

    pub comic_count: i32,
    pub comic_next_index: i32,

    pub created_at: i64,
    pub updated_at: i64,
}

/// Input parameters for creating a workset.
pub struct CreateWorksetData {
    pub team_id: String,

    pub name: String,
    pub description: Option<String>,
}

/// Return value from a successful workset creation.
pub struct CreateWorksetVal {
    pub id: String,
}

/// Input parameters for updating a workset's profile.
pub struct UpdateWorksetInfoData {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Input parameters for listing worksets.
pub struct ListWorksetInfosData {
    pub team_id: String,
}
