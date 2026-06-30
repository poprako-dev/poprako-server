//! Data transfer objects for chapter use cases.

use poprako_util::time::ToUnixMilli;

use crate::model::chapter::ChapterInfo;
use crate::value::chapter::{WorkflowEvent, WorkflowStage, WorkflowStageMask};

/// Presentation-ready chapter information.
///
/// Mirrors [`ChapterInfo`] but converts timestamps to Unix milliseconds
/// and exposes the same grouped field layout as the API response.
///
/// Construct via [`From<ChapterInfo>`] — the conversion is infallible.
///
/// [`ChapterInfo`]: crate::model::chapter::ChapterInfo
pub struct ChapterInfoVal {
    pub id: String,
    pub comic_id: String,

    pub is_pinned: bool,
    pub index: i32,
    pub subtitle: String,

    pub page_count: i32,
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,

    pub stages: WorkflowStageMask,

    pub creator_id: String,

    pub created_at: i64,
    pub updated_at: i64,
}

impl From<ChapterInfo> for ChapterInfoVal {
    fn from(model: ChapterInfo) -> Self {
        Self {
            id: model.id,
            comic_id: model.comic_id,
            is_pinned: model.is_pinned,
            index: model.index,
            subtitle: model.subtitle,
            page_count: model.page_count,
            total_unit_count: model.total_unit_count,
            translated_unit_count: model.translated_unit_count,
            proofread_unit_count: model.proofread_unit_count,
            stages: model.stages,
            creator_id: model.creator_id,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}

/// Input parameters for creating a new chapter.
pub struct CreateChapterData {
    pub comic_id: String,

    /// Optional display subtitle; defaults to a generated value
    /// if omitted (see [`default_subtitle`]).
    ///
    /// [`default_subtitle`]: crate::complex::chapter::default_subtitle
    pub subtitle: Option<String>,
}

/// Return value from a successful chapter creation.
pub struct CreateChapterVal {
    pub id: String,
}

/// Input parameters for listing chapters within a comic.
pub struct ListChapterInfosData {
    pub comic_id: String,

    pub offset: u64,
    pub limit: u64,
}

/// Input parameters for updating a chapter's profile.
pub struct UpdateChapterInfoData {
    pub id: String,

    pub subtitle: Option<String>,
    pub pin: Option<bool>,
}

/// Input parameters for updating a chapter's workflow stage.
///
/// Encodes a single event on a specific stage, e.g. "start translating"
/// on the `translate` stage. The use case layer validates that the
/// transition is legal for the current stage phase before applying it.
pub struct UpdateChapterStageData {
    pub id: String,

    pub stage: WorkflowStage,
    pub event: WorkflowEvent,
}
