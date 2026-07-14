//! Data transfer objects for workset use cases — input parameters and
//! presentation-ready values for the workset aggregate.
//!
//! Timestamps are converted to Unix milliseconds for JSON serialisation.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::{IntoParams, ToSchema};

use poprako_util::time::ToUnixMilli;

use crate::model::workset::WorksetInfo;

/// Presentation-ready workset information.
///
/// Mirrors [`WorksetInfo`] with timestamps converted to Unix milliseconds.
///
/// [`WorksetInfo`]: crate::model::workset::WorksetInfo
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
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

impl From<WorksetInfo> for WorksetInfoVal {
    fn from(model: WorksetInfo) -> Self {
        Self {
            id: model.id,
            team_id: model.team_id,
            index: model.index,
            name: model.name,
            description: model.description,
            comic_count: model.comic_count,
            comic_next_index: model.comic_next_index,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}

/// Input parameters for creating a new workset inside a team.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateWorksetParams {
    pub team_id: String,

    pub name: String,
    pub description: Option<String>,
}

/// Return value from a successful workset creation.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateWorksetPayload {
    pub id: String,
}

/// Input parameters for updating a workset's name and description.
///
/// Cover and counter updates are handled internally by the repo layer.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UpdateWorksetInfoParams {
    pub id: String,

    pub name: String,
    pub description: Option<String>,
}

/// Input parameters for listing worksets within a team.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(IntoParams))]
#[cfg_attr(feature = "swagger-ui", into_params(parameter_in = Query))]
pub struct ListWorksetInfosParams {
    pub team_id: String,

    pub offset: u32,
    pub limit: u32,
}
