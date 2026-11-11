//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::api::http::result::Wire;
use crate::data::instr::workset::{
    CreateWorksetInstr, ListWorksetInfosInstr, UpdateWorksetInfoInstr,
};
use crate::data::val::workset::CreateWorksetVal;
use crate::data::view::workset::WorksetInfoView;
use crate::value::pagination::PubListLimit;

/// Input parameters for creating a new workset inside a team.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateWorksetBody {
    /// Owning team identifier to create the workset under.
    pub team_id: String,

    /// Display name for the new workset.
    pub name: String,
    /// Optional description for the new workset.
    pub description: Option<String>,
}

impl From<CreateWorksetBody> for CreateWorksetInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateWorksetBody) -> Self {
        Self {
            team_id: value.team_id,
            name: value.name,
            description: value.description,
        }
    }
}

/// Input parameters for updating a workset's name and description.
///
/// Cover and counter updates are handled internally by the repo layer.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateWorksetInfoBody {
    /// Workset identifier to update.
    pub id: String,

    /// Updated workset display name.
    pub name: String,
    /// Updated workset description.
    pub description: Option<String>,
}

impl From<UpdateWorksetInfoBody> for UpdateWorksetInfoInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UpdateWorksetInfoBody) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
        }
    }
}

/// Input parameters for listing worksets within a team.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListWorksetInfosQuery {
    /// Owning team identifier to list worksets for.
    pub team_id: String,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

impl From<ListWorksetInfosQuery> for ListWorksetInfosInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListWorksetInfosQuery) -> Self {
        Self {
            team_id: value.team_id,
            offset: value.offset,
            limit: value.limit,
        }
    }
}

/// Return value from a successful workset creation.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateWorksetPayload {
    /// Identifier of the newly created workset.
    pub id: String,
}

impl From<CreateWorksetVal> for CreateWorksetPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateWorksetVal) -> Self {
        Self { id: value.id }
    }
}

impl Wire for CreateWorksetVal {
    // Serializable wire payload produced by this conversion.
    type Target = CreateWorksetPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Presentation-ready workset information.
///
/// Mirrors [`WorksetInfo`] with timestamps converted to Unix milliseconds.
///
/// [`WorksetInfo`]: crate::model::read::proj::workset::WorksetInfo
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct WorksetInfoObj {
    /// Unique workset identifier.
    pub id: String,

    /// Owning team identifier.
    pub team_id: String,

    /// Ordinal position of the workset within its team.
    pub index: usize,

    /// Workset display name.
    pub name: String,
    /// Optional description of the workset content or purpose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Total number of comics in this workset.
    pub comic_count: usize,

    /// Timestamp of workset creation, in milliseconds since Unix epoch.
    pub created_at: i64,
    /// Timestamp of the last workset update, in milliseconds since Unix epoch.
    pub updated_at: i64,
}

impl From<WorksetInfoView> for WorksetInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: WorksetInfoView) -> Self {
        Self {
            id: value.id,
            team_id: value.team_id,
            index: value.index,
            name: value.name,
            description: value.description,
            comic_count: value.comic_count,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl Wire for WorksetInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = WorksetInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}
