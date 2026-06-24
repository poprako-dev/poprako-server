//! Data transfer objects for workset use cases.

use poprako_util::time::ToUnixMilli;

use crate::model::workset::WorksetInfo as WorksetInfoModel;

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

impl From<WorksetInfoModel> for WorksetInfoVal {
    fn from(model: WorksetInfoModel) -> Self {
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

/// Input parameters for creating a workset.
pub struct WorksetCreateData {
    pub team_id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Return value from a successful workset creation.
pub struct WorksetCreateVal {
    pub workset: WorksetInfoVal,
}

/// Input parameters for updating a workset's profile.
pub struct WorksetInfoUpdateData {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Input parameters for listing worksets.
pub struct WorksetListData {
    pub team_id: String,
}
