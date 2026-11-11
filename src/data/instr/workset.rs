//! Instr DTOs for the workset domain.
//! Data transfer objects for workset use cases — input parameters and
//! presentation-ready values for the workset aggregate.
//!
//! Timestamps are converted to Unix milliseconds for JSON serialisation.
use crate::value::pagination::PubListLimit;

/// Input parameters for creating a new workset inside a team.
#[derive(Debug)]
pub struct CreateWorksetInstr {
    /// Owning team identifier to create the workset under.
    pub team_id: String,

    /// Display name for the new workset.
    pub name: String,
    /// Optional description for the new workset.
    pub description: Option<String>,
}

/// Input parameters for updating a workset's name and description.
///
/// Cover and counter updates are handled internally by the repo layer.
#[derive(Debug)]
pub struct UpdateWorksetInfoInstr {
    /// Workset identifier to update.
    pub id: String,

    /// Updated workset display name.
    pub name: String,
    /// Updated workset description.
    pub description: Option<String>,
}

/// Input parameters for listing worksets within a team.
#[derive(Debug)]
pub struct ListWorksetInfosInstr {
    /// Owning team identifier to list worksets for.
    pub team_id: String,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}
