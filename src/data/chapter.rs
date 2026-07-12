//! Data transfer objects for chapter use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::{IntoParams, ToSchema};

use poprako_util::time::ToUnixMilli;

use crate::data::{comic_data, user_data};
use crate::model::chapter_model;
use crate::part::image::ImagePool;
use crate::result::RegularResult;
use crate::value::chapter::{ChapterInclOpt, Stage, StageMask, StageOper};

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
pub struct InfoVal {
    pub id: String,
    pub comic_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "swagger-ui", schema(no_recursion))]
    pub comic: Option<Box<comic_data::InfoVal>>,

    pub is_pinned: bool,
    pub index: i32,
    pub subtitle: String,

    pub page_count: i32,
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,

    pub stages: StageMask,

    pub creator_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<user_data::InfoVal>,

    pub created_at: i64,
    pub updated_at: i64,
}

impl From<chapter_model::Info> for InfoVal {
    fn from(model: chapter_model::Info) -> Self {
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

impl InfoVal {
    /// Converts a chapter model into a presentation-ready value,
    /// resolving included creator avatar when present.
    pub async fn from_model<P>(
        image_pool: &P,
        model: chapter_model::Info,
    ) -> RegularResult<Self>
    where
        P: ImagePool,
    {
        let creator = match model.creator {
            //
            Some(user_info) => Some(
                user_data::InfoVal::from_model(image_pool, user_info).await?,
            ),

            None => None,
        };

        let comic = match model.comic {
            //
            Some(comic_info) => {
                //
                let comic = comic_data::InfoVal::from_model(
                    image_pool, comic_info, None,
                )
                .await?;

                Some(Box::new(comic))
            }

            None => None,
        };

        Ok(Self {
            id: model.id,
            comic_id: model.comic_id,
            comic,
            is_pinned: model.is_pinned,
            index: model.index,
            subtitle: model.subtitle,
            page_count: model.page_count,
            total_unit_count: model.total_unit_count,
            translated_unit_count: model.translated_unit_count,
            proofread_unit_count: model.proofread_unit_count,
            stages: model.stages,
            creator_id: model.creator_id,
            creator,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        })
    }
}

/// Input parameters for creating a new chapter.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateData {
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
pub struct CreateVal {
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
pub struct ListInfosData {
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
pub struct PatchInfoData {
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
pub struct UpdateStageData {
    pub id: String,

    pub stage: Stage,
    pub oper: StageOper,
}
