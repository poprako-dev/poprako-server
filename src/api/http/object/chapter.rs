//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::api::http::object::comic::ComicInfoObj;
use crate::api::http::object::user::UserInfoObj;
use crate::api::http::result::Wire;
use crate::data::instr::chapter::{
    ChapterStageInstr, ChapterStageOperInstr, CreateChapterInstr,
    ListChapterInfosInstr, ListChapterWorkflowRecordInfosInstr,
    UpdateChapterInfoInstr, UpdateChapterStageInstr,
};
use crate::data::val::chapter::CreateChapterVal;
use crate::data::view::chapter::ChapterInfoView;
use crate::value::chapter::ChapterInclOpt;
use crate::value::chapter::mask::StageMask;
use crate::value::pagination::PubListLimit;
use crate::value::role::RoleMask;

/// Input parameters for creating a new chapter.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateChapterBody {
    /// Identifier of the parent comic to create the chapter in.
    pub comic_id: String,

    /// Optional display subtitle; defaults to a generated value
    /// if omitted (see [`default_subtitle`]).
    ///
    /// [`default_subtitle`]: crate::complex::chapter::default_subtitle
    pub subtitle: Option<String>,

    /// Optional worker roles assigned to the creator; absent means no assignment.
    /// Every requested role must exist on the creator's team membership.
    pub preset_assignment_roles: Option<RoleMask>,
}

impl From<CreateChapterBody> for CreateChapterInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateChapterBody) -> Self {
        Self {
            comic_id: value.comic_id,
            subtitle: value.subtitle,
            preset_assignment_roles: value.preset_assignment_roles,
        }
    }
}

/// Input parameters for listing chapters within a comic.
///
/// `incl` embeds related rows into each item; dotted values implicitly pull
/// in their parent segments.
///
/// Example: `/api/v1/comics/{comic_id}/chapters?incl=comic.workset.team&incl=creator&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListChapterInfosQuery {
    /// Parent comic whose chapters to list.
    pub comic_id: String,

    /// Related rows to embed. Repeatable. Values: `comic`, `comic.workset`,
    /// `comic.workset.team`, `comic.creator`, `creator`. Dotted values imply
    /// their parent segments.
    #[serde(default)]
    pub incl: Vec<ChapterInclOpt>,

    /// Pagination offset: number of chapters to skip before beginning the
    /// result set.
    pub offset: u32,
    /// Maximum number of chapters to return.
    pub limit: PubListLimit,
}

impl From<ListChapterInfosQuery> for ListChapterInfosInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListChapterInfosQuery) -> Self {
        Self {
            comic_id: value.comic_id,
            incl: value.incl,
            offset: value.offset,
            limit: value.limit,
        }
    }
}

/// Input parameters for listing immutable workflow records under one chapter.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListChapterWorkflowRecordInfosQuery {
    /// Chapter whose activity records are listed.
    pub chapter_id: String,
    /// Pagination offset: number of records to skip before beginning the result set.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: PubListLimit,
}

impl From<ListChapterWorkflowRecordInfosQuery>
    for ListChapterWorkflowRecordInfosInstr
{
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListChapterWorkflowRecordInfosQuery) -> Self {
        Self {
            chapter_id: value.chapter_id,
            offset: value.offset,
            limit: value.limit,
        }
    }
}

/// Input parameters for partially updating a chapter's profile.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateChapterInfoBody {
    /// Chapter identifier to update.
    pub id: String,

    /// New display subtitle; `None` leaves the current value unchanged.
    pub subtitle: Option<String>,
}

impl From<UpdateChapterInfoBody> for UpdateChapterInfoInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UpdateChapterInfoBody) -> Self {
        Self {
            id: value.id,
            subtitle: value.subtitle,
        }
    }
}

/// Input parameters for updating a chapter's workflow stage.
///
/// Encodes a single operation on a specific stage, e.g. "start translating"
/// on the `translate` stage. The use case layer validates that the
/// transition is legal for the current stage phase before applying it.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ChapterStageBody {
    /// Raw-provision stage.
    RawProvide,

    /// Translation stage.
    Translate,

    /// Proofreading stage.
    Proofread,

    /// Typesetting and redraw stage.
    TypesetRedraw,

    /// Review stage.
    Review,

    /// Publishing stage.
    Publish,
}

impl From<ChapterStageBody> for ChapterStageInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ChapterStageBody) -> Self {
        //
        match value {
            //
            ChapterStageBody::RawProvide => Self::RawProvide,

            ChapterStageBody::Translate => Self::Translate,

            ChapterStageBody::Proofread => Self::Proofread,

            ChapterStageBody::TypesetRedraw => Self::TypesetRedraw,

            ChapterStageBody::Review => Self::Review,

            ChapterStageBody::Publish => Self::Publish,
        }
    }
}

/// Operation accepted by the workflow-stage JSON body.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ChapterStageOperBody {
    /// Advance the stage.
    Advance,

    /// Revert the stage.
    Revert,
}

impl From<ChapterStageOperBody> for ChapterStageOperInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ChapterStageOperBody) -> Self {
        //
        match value {
            //
            ChapterStageOperBody::Advance => Self::Advance,

            ChapterStageOperBody::Revert => Self::Revert,
        }
    }
}

/// Input parameters for applying one chapter workflow-stage operation.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateChapterStageBody {
    /// Chapter identifier to update.
    pub id: String,

    /// Workflow stage to operate on.
    pub stage: ChapterStageBody,
    /// Operation to apply to the target stage (e.g. start, finish, revert).
    pub oper: ChapterStageOperBody,
}

impl From<UpdateChapterStageBody> for UpdateChapterStageInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UpdateChapterStageBody) -> Self {
        Self {
            id: value.id,
            stage: value.stage.into(),
            oper: value.oper.into(),
        }
    }
}

/// Return value from a successful chapter creation.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateChapterPayload {
    /// Unique identifier of the newly created chapter.
    pub id: String,
}

impl From<CreateChapterVal> for CreateChapterPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateChapterVal) -> Self {
        Self { id: value.id }
    }
}

impl Wire for CreateChapterVal {
    // Serializable wire payload produced by this conversion.
    type Target = CreateChapterPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Presentation-ready chapter information.
///
/// Mirrors [`ChapterInfo`] but converts timestamps to Unix milliseconds
/// and exposes the same grouped field layout as the API response.
///
/// Construct via [`From<ChapterInfo>`] — the conversion is infallible.
///
/// [`ChapterInfo`]: crate::model::read::proj::chapter::ChapterInfo
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ChapterInfoObj {
    /// Unique chapter identifier.
    pub id: String,

    /// Owning comic identifier.
    pub comic_id: String,

    /// Included comic information; present only when requested via inclusion
    /// options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comic: Option<ComicInfoObj>,

    /// Whether the chapter is pinned to the top of its comic's chapter list.
    pub is_pinned: bool,

    /// Ordinal position within the parent comic.
    pub index: usize,
    /// Display subtitle for the chapter.
    pub subtitle: String,

    /// Total number of image pages in the chapter.
    pub page_source_count: usize,
    /// Total number of translation units across all pages.
    pub total_unit_count: usize,
    /// Number of units whose translation has been completed.
    pub translated_unit_count: usize,
    /// Number of units whose proofread has been completed.
    pub proofread_unit_count: usize,

    /// Bitmask encoding the current workflow-stage states.
    pub stages: StageMask,

    /// Creator user identifier.
    pub creator_id: String,

    /// Included creator information; present only when requested via inclusion
    /// options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<UserInfoObj>,

    /// Timestamp of creation, in Unix milliseconds.
    pub created_at: i64,
    /// Timestamp of last update, in Unix milliseconds.
    pub updated_at: i64,
}

impl From<ChapterInfoView> for ChapterInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ChapterInfoView) -> Self {
        Self {
            id: value.id,
            comic_id: value.comic_id,
            comic: value.comic.map(std::convert::Into::into),
            is_pinned: value.is_pinned,
            index: value.index,
            subtitle: value.subtitle,
            page_source_count: value.page_source_count,
            total_unit_count: value.total_unit_count,
            translated_unit_count: value.translated_unit_count,
            proofread_unit_count: value.proofread_unit_count,
            stages: value.stages,
            creator_id: value.creator_id,
            creator: value.creator.map(std::convert::Into::into),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl Wire for ChapterInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = ChapterInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Query for listing immutable chapter workflow records.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ChapterWorkflowRecordListQuery {
    /// Pagination offset (0-based).
    pub offset: u32,
    /// Maximum number of items to return.
    pub limit: PubListLimit,
}

/// Query for listing chapters within a comic.
///
/// `incl` embeds related rows into each item; dotted values implicitly pull
/// in their parent segments.
///
/// Example: `?incl=comic.workset.team&incl=creator&offset=0&limit=20`.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ChapterListQuery {
    /// Related rows to embed. Repeatable. Values: `comic`, `comic.workset`,
    /// `comic.workset.team`, `comic.creator`, `creator`. Dotted values imply
    /// their parent segments.
    #[serde(default)]
    pub incl: Vec<ChapterInclOpt>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: PubListLimit,
}
