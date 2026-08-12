//! Instr DTOs for the termbase domain.

//! Request and response DTOs for terminology-base use cases.

use serde::Deserialize;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Input parameters for creating a terminology base.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateTermbaseInstr {
    /// Team scope identifier; absent for comic-scoped termbases.
    pub team_id: Option<String>,
    /// Comic scope identifier; absent for team-scoped termbases.
    pub comic_id: Option<String>,

    /// Human-readable name for the new terminology base.
    pub name: String,
    /// Optional longer description.
    pub description: Option<String>,
}

/// Input parameters for replacing terminology-base profile fields.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateTermbaseInfoInstr {
    /// Terminology-base identifier to update.
    pub id: String,

    /// Updated human-readable name for the terminology base.
    pub name: String,
    /// Updated description for the terminology base.
    pub description: Option<String>,
}

/// Input parameters for listing team-owned terminology bases.
#[derive(Debug)]
pub struct ListTeamTermbaseInfosInstr {
    /// Owning team identifier.
    pub team_id: String,

    /// Optional fuzzy name filter for termbase search.
    pub fuzzy_name: Option<String>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}

/// Input parameters for listing terminology bases visible from a comic.
#[derive(Debug)]
pub struct ListComicTermbaseInfosInstr {
    /// Owning comic identifier.
    pub comic_id: String,

    /// Optional fuzzy name filter for termbase search.
    pub fuzzy_name: Option<String>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}
