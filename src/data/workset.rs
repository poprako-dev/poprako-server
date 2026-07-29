//! Data transfer objects for workset use cases — input parameters and
//! presentation-ready values for the workset aggregate.
//!
//! Timestamps are converted to Unix milliseconds for JSON serialisation.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use poprako_util::time::ToUnixMilli;

use crate::model::workset::WorksetInfo;

/// Presentation-ready workset information.
///
/// Mirrors [`WorksetInfo`] with timestamps converted to Unix milliseconds.
///
/// [`WorksetInfo`]: crate::model::workset::WorksetInfo
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct WorksetInfoVal {
    /// Unique workset identifier.
    pub id: String,
    /// Owning team identifier.
    pub team_id: String,

    /// Ordinal position of the workset within its team.
    pub index: i32,

    /// Workset display name.
    pub name: String,
    /// Optional description of the workset content or purpose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Total number of comics in this workset.
    pub comic_count: i32,

    /// Timestamp of workset creation, in milliseconds since Unix epoch.
    pub created_at: i64,
    /// Timestamp of the last workset update, in milliseconds since Unix epoch.
    pub updated_at: i64,
}

impl From<WorksetInfo> for WorksetInfoVal {
    fn from(model: WorksetInfo) -> Self {
        Self {
            id: model.id,
            team_id: model.team_id,
            index: model.index,
            name: model.name,
            description: model.description,
            comic_count: model.comic_count,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}

/// Input parameters for creating a new workset inside a team.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateWorksetParams {
    /// Owning team identifier to create the workset under.
    pub team_id: String,

    /// Display name for the new workset.
    pub name: String,
    /// Optional description for the new workset.
    pub description: Option<String>,
}

/// Return value from a successful workset creation.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateWorksetPayload {
    /// Identifier of the newly created workset.
    pub id: String,
}

/// Input parameters for updating a workset's name and description.
///
/// Cover and counter updates are handled internally by the repo layer.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateWorksetInfoParams {
    /// Workset identifier to update.
    pub id: String,

    /// Updated workset display name.
    pub name: String,
    /// Updated workset description.
    pub description: Option<String>,
}

/// Input parameters for listing worksets within a team.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListWorksetInfosParams {
    /// Owning team identifier to list worksets for.
    pub team_id: String,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}
