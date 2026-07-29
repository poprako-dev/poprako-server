//! Request and response DTOs for terminology-base use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::model::termbase::TermbaseInfo;

/// Presentation-ready terminology-base information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct TermbaseInfoVal {
    //
    /// Unique terminology-base identifier.
    pub id: String,

    /// Owning team identifier; absent when the termbase is comic-scoped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    /// Owning comic identifier; absent when the termbase is team-scoped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comic_id: Option<String>,

    /// Human-readable terminology-base name.
    pub name: String,
    /// Optional longer description of the terminology base.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Number of terms in the terminology base.
    pub term_count: i32,

    /// Identifier of the user who created the terminology base.
    pub creator_id: String,

    /// Timestamp of creation, in milliseconds since Unix epoch.
    pub created_at: i64,
    /// Timestamp of last update, in milliseconds since Unix epoch.
    pub updated_at: i64,
}

impl From<TermbaseInfo> for TermbaseInfoVal {
    fn from(model: TermbaseInfo) -> Self {
        Self {
            id: model.id,
            team_id: model.team_id,
            comic_id: model.comic_id,
            name: model.name,
            description: model.description,
            term_count: model.term_count,
            creator_id: model.creator_id,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}

/// Input parameters for creating a terminology base.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateTermbaseParams {
    //
    /// Team scope identifier; absent for comic-scoped termbases.
    pub team_id: Option<String>,
    /// Comic scope identifier; absent for team-scoped termbases.
    pub comic_id: Option<String>,

    /// Human-readable name for the new terminology base.
    pub name: String,
    /// Optional longer description.
    pub description: Option<String>,
}

/// Return value from creating a terminology base.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateTermbasePayload {
    /// Identifier of the newly created terminology base.
    pub id: String,
}

/// Input parameters for replacing terminology-base profile fields.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateTermbaseInfoParams {
    //
    /// Terminology-base identifier to update.
    pub id: String,

    /// Updated human-readable name for the terminology base.
    pub name: String,
    /// Updated description for the terminology base.
    pub description: Option<String>,
}

/// Input parameters for listing team-owned terminology bases.
pub struct ListTeamTermbaseInfosParams {
    //
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
pub struct ListComicTermbaseInfosParams {
    //
    /// Owning comic identifier.
    pub comic_id: String,

    /// Optional fuzzy name filter for termbase search.
    pub fuzzy_name: Option<String>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}
