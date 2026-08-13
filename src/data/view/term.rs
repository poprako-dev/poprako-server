//! View DTOs for the term domain.

use serde::Serialize;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::model::read::proj::term::TermInfo;

/// Presentation-ready terminology-entry information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct TermInfoView {
    /// Unique identifier of the terminology entry.
    pub id: String,

    /// Parent terminology base identifier.
    pub termbase_id: String,

    /// Source-language term text.
    pub source: String,
    /// Target-language translations.
    pub targets: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional annotation; absent when the entry has no comment.
    pub comment: Option<String>,

    /// Identifier of the user who created this entry.
    pub creator_id: String,

    /// Unix timestamp in milliseconds of creation.
    pub created_at: i64,
    /// Unix timestamp in milliseconds of the last update.
    pub updated_at: i64,
}

impl From<TermInfo> for TermInfoView {
    // Convert terminology entry persistence model into response value.
    fn from(model: TermInfo) -> Self {
        //
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
