//! View DTOs for the workset domain.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::model::read::proj::workset::WorksetInfo;

/// Presentation-ready workset information.
///
/// Mirrors [`WorksetInfo`] with timestamps converted to Unix milliseconds.
///
/// [`WorksetInfo`]: crate::model::read::proj::workset::WorksetInfo
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct WorksetInfoView {
    //
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

impl From<WorksetInfo> for WorksetInfoView {
    // Flatten workset persistence model into API response form.
    fn from(model: WorksetInfo) -> Self {
        //
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
