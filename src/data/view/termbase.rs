//! View DTOs for the termbase domain.

use serde::Serialize;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::model::read::proj::termbase::TermbaseInfo;

/// Presentation-ready terminology-base information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct TermbaseInfoView {
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

impl From<TermbaseInfo> for TermbaseInfoView {
    // Convert terminology base model into response payload.
    fn from(model: TermbaseInfo) -> Self {
        //
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
