//! Val DTOs for the assignment domain.

//! Data transfer objects for assignment use cases.

use serde::Serialize;

use crate::data::val::chapter::ChapterInfoVal;
use crate::data::val::user::UserInfoVal;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use futures::future::OptionFuture;

use poprako_util::time::ToUnixMilli;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::part::image::ImagePool;
use crate::result::{BaseRest, accept};
use crate::value::role::RoleMask;

/// Presentation-ready chapter assignment information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AssignmentInfoVal {
    //
    /// Unique identifier of the assignment.
    pub id: String,

    /// Owning chapter identifier.
    pub chapter_id: String,
    /// Assigned user identifier.
    pub user_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Resolved user information, when included.
    pub user: Option<UserInfoVal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Resolved chapter information, when included.
    pub chapter: Option<ChapterInfoVal>,

    /// Role mask assigned to this user for the chapter.
    pub roles: RoleMask,

    /// Timestamp of creation in milliseconds.
    pub created_at: i64,
    /// Timestamp of last update in milliseconds.
    pub updated_at: i64,
}

impl AssignmentInfoVal {
    /// Build a response value and eagerly resolve included user/chapter references.
    /// Converts an assignment model into a presentation-ready value,
    /// resolving included user avatar when present.
    pub async fn from_model<P>(
        image_pool: &P,
        model: AssignmentInfo,
        fallback_cover_key: Option<&str>,
    ) -> BaseRest<Self>
    where
        P: ImagePool,
    {
        accept(Self {
            id: model.id,
            chapter_id: model.chapter_id,
            user_id: model.user_id,
            user: OptionFuture::from(model.user.map(|user_info| {
                UserInfoVal::from_model(image_pool, user_info)
            }))
            .await
            .transpose()?,
            chapter: OptionFuture::from(model.chapter.map(|chapter_info| {
                ChapterInfoVal::from_model(
                    image_pool,
                    chapter_info,
                    fallback_cover_key,
                )
            }))
            .await
            .transpose()?,
            roles: model.roles,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        })
    }
}

impl From<AssignmentInfo> for AssignmentInfoVal {
    // Convert one persisted assignment into API value shape.
    fn from(model: AssignmentInfo) -> Self {
        Self {
            id: model.id,
            chapter_id: model.chapter_id,
            user_id: model.user_id,
            user: None,
            chapter: None,
            roles: model.roles,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}
