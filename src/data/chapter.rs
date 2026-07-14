//! Data transfer objects for chapter use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::{IntoParams, ToSchema};

use poprako_util::time::ToUnixMilli;

use crate::data::comic::ComicInfoVal;

use crate::data::user::UserInfoVal;
use crate::model::chapter::ChapterInfo;
use crate::part::image::ImagePool;
use crate::result::RegularResult;
use crate::value::chapter::{ChapterInclOpt, Stage, StageMask, StageOper};
use futures::future::OptionFuture;

/// Presentation-ready chapter information.
///
/// Mirrors [`ChapterInfo`] but converts timestamps to Unix milliseconds
/// and exposes the same grouped field layout as the API response.
///
/// Construct via [`From<ChapterInfo>`] — the conversion is infallible.
///
/// [`ChapterInfo`]: crate::model::chapter::ChapterInfo
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ChapterInfoVal {
    pub id: String,
    pub comic_id: String,

    pub comic: Option<ComicInfoVal>,

    pub is_pinned: bool,
    pub index: i32,
    pub subtitle: String,

    pub page_count: i32,
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,

    pub stages: StageMask,

    pub creator_id: String,

    pub creator: Option<UserInfoVal>,

    pub created_at: i64,
    pub updated_at: i64,
}

impl From<ChapterInfo> for ChapterInfoVal {
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

impl ChapterInfoVal {
    /// Converts a chapter model into a presentation-ready value,
    /// resolving included creator avatar when present.
    pub async fn from_model<P>(
        image_pool: &P,
        model: ChapterInfo,
        fallback_cover_key: Option<&str>,
    ) -> RegularResult<Self>
    where
        P: ImagePool,
    {
        Ok(Self {
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

/// Input parameters for creating a new chapter.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateChapterParams {
    pub comic_id: String,

    /// Optional display subtitle; defaults to a generated value
    /// if omitted (see [`default_subtitle`]).
    ///
    /// [`default_subtitle`]: crate::complex::chapter::default_subtitle
    pub subtitle: Option<String>,
}

/// Return value from a successful chapter creation.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateChapterPayload {
    pub id: String,
}

/// Input parameters for listing chapters within a comic.
///
/// `incl` embeds related rows into each item; dotted values implicitly pull
/// in their parent segments.
///
/// Example: `/api/v1/comics/{comic_id}/chapters?incl=comic.workset.team&incl=creator&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(IntoParams))]
#[cfg_attr(feature = "swagger-ui", into_params(parameter_in = Query))]
pub struct ListChapterInfosParams {
    /// Parent comic whose chapters to list.
    pub comic_id: String,

    /// Related rows to embed. Repeatable. Values: `comic`, `comic.workset`,
    /// `comic.workset.team`, `comic.creator`, `creator`. Dotted values imply
    /// their parent segments.
    #[serde(
        default,
        rename = "incl",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub incl_opt: Vec<ChapterInclOpt>,

    pub offset: u32,
    pub limit: u32,
}

/// Input parameters for partially updating a chapter's profile.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UpdateChapterInfoParams {
    pub id: String,

    pub subtitle: Option<String>,
    pub pin: Option<bool>,
}

/// Input parameters for updating a chapter's workflow stage.
///
/// Encodes a single operation on a specific stage, e.g. "start translating"
/// on the `translate` stage. The use case layer validates that the
/// transition is legal for the current stage phase before applying it.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UpdateChapterStageParams {
    pub id: String,

    pub stage: Stage,
    pub oper: StageOper,
}
