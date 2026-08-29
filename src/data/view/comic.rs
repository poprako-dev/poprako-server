//! View DTOs for the comic domain.

#[cfg(test)]
mod tests;

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::data::view::team::TeamInfoView;
use crate::data::view::user::UserInfoView;
use crate::data::view::workset::WorksetInfoView;
use crate::model::read::proj::comic::ComicInfo;

/// Presentation-ready comic information.
///
/// Mirrors [`ComicInfo`], converts timestamps to Unix milliseconds, and
/// accepts object URLs already resolved by the use-case layer.
///
/// [`ComicInfo`]: crate::model::read::proj::comic::ComicInfo
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ComicInfoView {
    /// Unique comic identifier.
    pub id: String,

    /// Parent workset identifier this comic belongs to.
    pub workset_id: String,
    /// Ordinal position of the comic within its workset.
    pub index: usize,

    /// Comic title.
    pub title: String,
    /// Comic author name.
    pub author: String,
    /// Optional description or synopsis of the comic content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Resolved signed download URL for the cover image, or [`None`] if
    /// no cover has been uploaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    /// Total number of chapters in this comic.
    pub chapter_count: usize,

    /// Identifier of the user who created the comic entry.
    pub creator_id: String,

    /// Resolved workset summary, included when the `workset` expansion option is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workset: Option<WorksetInfoView>,
    /// Resolved team summary for the owning workset, included when the `team` expansion option is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<TeamInfoView>,
    /// Resolved creator profile, included when the `creator` expansion option is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<UserInfoView>,

    /// Timestamp of the most recent activity on the comic, in milliseconds since Unix epoch.
    pub last_active_at: i64,

    /// Whether this comic has been archived and is no longer writable.
    pub is_archived: bool,
    /// Timestamp when the comic was archived, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<i64>,

    /// Timestamp of comic creation, in milliseconds since Unix epoch.
    pub created_at: i64,
    /// Timestamp of the last comic update, in milliseconds since Unix epoch.
    pub updated_at: i64,
}

impl ComicInfoView {
    /// Converts a [`ComicInfo`] into a presentation-ready value.
    ///
    /// Accepts the resolved cover URL and converts timestamps from
    /// [`OffsetDateTime`] to Unix milliseconds.
    ///
    /// [`OffsetDateTime`]: time::OffsetDateTime
    pub fn from_model(
        model: ComicInfo,
        cover_url: Option<String>,
        team: Option<TeamInfoView>,
        creator: Option<UserInfoView>,
    ) -> Self {
        //
        let workset = model.workset.map(WorksetInfoView::from);

        Self {
            id: model.id,
            workset_id: model.workset_id,
            index: model.index,
            title: model.title,
            author: model.author,
            description: model.description,
            cover_url,
            chapter_count: model.chapter_count,
            creator_id: model.creator_id,
            workset,
            team,
            creator,
            last_active_at: model.last_active_at.to_unix_milli(),
            is_archived: model.archived_at.is_some(),
            archived_at: model
                .archived_at
                .map(|archived_at| archived_at.to_unix_milli()),
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}
