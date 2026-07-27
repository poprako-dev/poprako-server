//! Data transfer objects for chapter use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use futures::future::OptionFuture;

use poprako_util::time::ToUnixMilli;

use crate::data::comic::ComicInfoVal;
use crate::data::user::UserInfoVal;
use crate::model::chapter::ChapterInfo;
use crate::part::image::ImagePool;
use crate::result::{BaseResult, accept};
use crate::value::chapter::{ChapterInclOpt, Stage, StageMask, StageOper};
use crate::value::role::RoleMask;

/// Presentation-ready chapter information.
///
/// Mirrors [`ChapterInfo`] but converts timestamps to Unix milliseconds
/// and exposes the same grouped field layout as the API response.
///
/// Construct via [`From<ChapterInfo>`] — the conversion is infallible.
///
/// [`ChapterInfo`]: crate::model::chapter::ChapterInfo
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ChapterInfoVal {
    //
    /// Unique chapter identifier.
    pub id: String,
    /// Owning comic identifier.
    pub comic_id: String,

    /// Included comic information; present only when requested via inclusion
    /// options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comic: Option<ComicInfoVal>,

    /// Whether the chapter is pinned to the top of its comic's chapter list.
    pub is_pinned: bool,
    /// Ordinal position within the parent comic.
    pub index: i32,
    /// Display subtitle for the chapter.
    pub subtitle: String,

    /// Total number of image pages in the chapter.
    pub page_count: i32,
    /// Total number of translation units across all pages.
    pub total_unit_count: i32,
    /// Number of units whose translation has been completed.
    pub translated_unit_count: i32,
    /// Number of units whose proofread has been completed.
    pub proofread_unit_count: i32,

    /// Bitmask encoding the current workflow-stage states.
    pub stages: StageMask,

    /// Creator user identifier.
    pub creator_id: String,

    /// Included creator information; present only when requested via inclusion
    /// options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<UserInfoVal>,

    /// Timestamp of creation, in Unix milliseconds.
    pub created_at: i64,
    /// Timestamp of last update, in Unix milliseconds.
    pub updated_at: i64,
}

impl ChapterInfoVal {
    /// Converts a chapter model into a presentation-ready value,
    /// resolving included creator avatar when present.
    pub async fn from_model<P>(
        image_pool: &P,
        model: ChapterInfo,
        fallback_cover_key: Option<&str>,
    ) -> BaseResult<Self>
    where
        P: ImagePool,
    {
        accept(Self {
            id: model.id,
            comic_id: model.comic_id,
            comic: OptionFuture::from(model.comic.map(|comic_info| {
                ComicInfoVal::from_model(
                    image_pool,
                    comic_info,
                    fallback_cover_key,
                )
            }))
            .await
            .transpose()?,
            is_pinned: model.is_pinned,
            index: model.index,
            subtitle: model.subtitle,
            page_count: model.page_count,
            total_unit_count: model.total_unit_count,
            translated_unit_count: model.translated_unit_count,
            proofread_unit_count: model.proofread_unit_count,
            stages: model.stages,
            creator_id: model.creator_id,
            creator: OptionFuture::from(model.creator.map(|user_info| {
                UserInfoVal::from_model(image_pool, user_info)
            }))
            .await
            .transpose()?,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        })
    }
}

impl From<ChapterInfo> for ChapterInfoVal {
    // Copy persisted chapter fields into the API value shape.
    fn from(model: ChapterInfo) -> Self {
        Self {
            id: model.id,
            comic_id: model.comic_id,
            comic: None,
            is_pinned: model.is_pinned,
            index: model.index,
            subtitle: model.subtitle,
            page_count: model.page_count,
            total_unit_count: model.total_unit_count,
            translated_unit_count: model.translated_unit_count,
            proofread_unit_count: model.proofread_unit_count,
            stages: model.stages,
            creator_id: model.creator_id,
            creator: None,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}

/// Input parameters for creating a new chapter.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateChapterParams {
    //
    /// Identifier of the parent comic to create the chapter in.
    pub comic_id: String,

    /// Optional display subtitle; defaults to a generated value
    /// if omitted (see [`default_subtitle`]).
    ///
    /// [`default_subtitle`]: crate::complex::chapter::default_subtitle
    pub subtitle: Option<String>,

    /// Roles assigned to the creator in addition to the mandatory admin role.
    /// Every requested role must exist on the creator's team membership.
    pub preset_assignment_roles: Option<RoleMask>,
}

/// Return value from a successful chapter creation.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateChapterPayload {
    /// Unique identifier of the newly created chapter.
    pub id: String,
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
pub struct ListChapterInfosParams {
    //
    /// Parent comic whose chapters to list.
    pub comic_id: String,

    /// Related rows to embed. Repeatable. Values: `comic`, `comic.workset`,
    /// `comic.workset.team`, `comic.creator`, `creator`. Dotted values imply
    /// their parent segments.
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<ChapterInclOpt>,

    /// Pagination offset: number of chapters to skip before beginning the
    /// result set.
    pub offset: u32,
    /// Maximum number of chapters to return.
    pub limit: u32,
}

/// Input parameters for partially updating a chapter's profile.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateChapterInfoParams {
    //
    /// Chapter identifier to update.
    pub id: String,

    /// New display subtitle; `None` leaves the current value unchanged.
    pub subtitle: Option<String>,
    /// New pin status; `None` leaves the current value unchanged.
    pub pin: Option<bool>,
}

/// Input parameters for updating a chapter's workflow stage.
///
/// Encodes a single operation on a specific stage, e.g. "start translating"
/// on the `translate` stage. The use case layer validates that the
/// transition is legal for the current stage phase before applying it.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateChapterStageParams {
    //
    /// Chapter identifier to update.
    pub id: String,

    /// Workflow stage to operate on.
    pub stage: Stage,
    /// Operation to apply to the target stage (e.g. start, finish, revert).
    pub oper: StageOper,
}
