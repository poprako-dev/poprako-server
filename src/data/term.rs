//! Request and response DTOs for terminology-entry use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::model::term::TermInfo;

/// Presentation-ready terminology-entry information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct TermInfoVal {
    pub id: String,

    pub termbase_id: String,

    pub source: String,
    pub targets: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    pub creator_id: String,

    pub created_at: i64,
    pub updated_at: i64,
}

impl From<TermInfo> for TermInfoVal {
    fn from(model: TermInfo) -> Self {
        Self {
            id: model.id,
            termbase_id: model.termbase_id,
            source: model.source,
            targets: model.targets,
            comment: model.comment,
            creator_id: model.creator_id,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}

/// Input parameters for creating a terminology entry.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateTermParams {
    pub termbase_id: String,

    pub source: String,
    pub targets: Vec<String>,
    pub comment: Option<String>,
}

/// Return value from creating a terminology entry.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateTermPayload {
    pub id: String,
}

/// Input parameters for replacing terminology-entry fields.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UpdateTermInfoParams {
    pub id: String,

    pub source: String,
    pub targets: Vec<String>,
    pub comment: Option<String>,
}

/// Input parameters for listing terms inside one terminology base.
pub struct ListTermInfosParams {
    pub termbase_id: String,

    pub fuzzy_source: Option<String>,

    pub offset: u32,
    pub limit: u32,
}
