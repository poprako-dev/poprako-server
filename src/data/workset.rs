//! Data transfer objects for workset use cases — input parameters and
//! presentation-ready values for the workset aggregate.
//!
//! Timestamps are converted to Unix milliseconds for JSON serialisation.

use poprako_macro::Paginate;

/// Presentation-ready workset information.
///
/// Mirrors [`WorksetInfo`] with timestamps converted to Unix milliseconds.
///
/// [`WorksetInfo`]: crate::model::workset::WorksetInfo
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

/// Input parameters for creating a new workset inside a team.
pub struct CreateWorksetData {
    pub team_id: String,

    pub name: String,
    pub description: Option<String>,
}

/// Return value from a successful workset creation.
pub struct CreateWorksetVal {
    pub id: String,
}

/// Input parameters for updating a workset's name and description.
///
/// Cover and counter updates are handled internally by the repo layer.
pub struct UpdateWorksetInfoData {
    pub id: String,

    pub name: String,
    pub description: Option<String>,
}

/// Input parameters for listing worksets within a team.
#[Paginate]
pub struct ListWorksetInfosData {
    pub team_id: String,
}
