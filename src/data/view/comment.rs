//! View DTOs for the comment domain.

use futures::future::OptionFuture;
use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::data::view::user::UserInfoView;
use crate::model::read::proj::comment::CommentInfo;
use crate::part::image::ImagePool;
use crate::result::{BaseRest, accept};

/// Presentation-ready team board comment information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CommentInfoView {
    //
    /// Unique comment identifier.
    pub id: String,

    /// Owning team identifier.
    pub team_id: String,
    /// Authoring user identifier.
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Author user information, present when requested via inclusion options.
    pub user: Option<UserInfoView>,

    /// Comment body text.
    pub content: String,

    /// Timestamp of creation in milliseconds since Unix epoch.
    pub created_at: i64,
}

impl CommentInfoView {
    /// Convert a persisted comment row into API output with optional author include.
    /// Converts a comment model into a presentation value.
    pub async fn from_model<P>(
        image_pool: &P,
        model: CommentInfo,
    ) -> BaseRest<Self>
    where
        P: ImagePool + Sync,
    {
        accept(Self {
            id: model.id,
            team_id: model.team_id,
            user_id: model.user_id,
            user: OptionFuture::from(model.user.map(|user_info| {
                UserInfoView::from_model(image_pool, user_info)
            }))
            .await
            .transpose()?,
            content: model.content,
            created_at: model.created_at.to_unix_milli(),
        })
    }
}
