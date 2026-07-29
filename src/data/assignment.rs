//! Data transfer objects for assignment use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use futures::future::OptionFuture;

use poprako_util::i18n::trl;
use poprako_util::time::ToUnixMilli;

use crate::data::chapter::ChapterInfoVal;
use crate::data::user::UserInfoVal;
use crate::model::assignment::{AssignmentInfo, AssignmentInfoListSpec};
use crate::part::image::ImagePool;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::value::assignment::AssignmentInclOpt;
use crate::value::role::{RoleField, RoleMask};

/// Presentation-ready chapter assignment information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AssignmentInfoVal {
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

impl From<AssignmentInfo> for AssignmentInfoVal {
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

impl AssignmentInfoVal {
    /// Converts an assignment model into a presentation-ready value,
    /// resolving included user avatar when present.
    pub async fn from_model<P>(
        image_pool: &P,
        model: AssignmentInfo,
        fallback_cover_key: Option<&str>,
    ) -> BaseResult<Self>
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

/// Input parameters for listing assignments by chapter or owner user.
///
/// Exactly one of `chapter_id` or `owner_id` is required:
/// - `chapter_id`: list assignments on that chapter;
/// - `owner_id`: list assignments owned by that user.
///
/// `role` optionally narrows by a single role bit in either mode. `incl`
/// embeds related rows; dotted values imply their parent segments.
///
/// Example: `/api/v1/assignments?chapter_id=c_1&role=1&incl=chapter.comic.workset.team&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListAssignmentInfosParams {
    /// Chapter mode: list assignments on this chapter. Mutually exclusive with
    /// `owner_id`.
    pub chapter_id: Option<String>,

    /// Owner-user mode: list assignments owned by this user. Mutually exclusive
    /// with `chapter_id`.
    pub owner_id: Option<String>,

    /// Single role-bit filter. Must be a singular valid role bit; composite
    /// values are rejected.
    pub role: Option<RoleField>,

    /// Related rows to embed. Repeatable. Values: `user`, `chapter`,
    /// `chapter.comic`, `chapter.comic.workset`, `chapter.comic.workset.team`,
    /// `chapter.creator`, `chapter.comic.creator`. Dotted values imply their
    /// parent segments.
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<AssignmentInclOpt>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}

impl TryInto<AssignmentInfoListSpec> for ListAssignmentInfosParams {
    type Error = BaseError;

    fn try_into(self) -> BaseResult<AssignmentInfoListSpec> {
        //
        let invalid_args = || BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-chapter-or-user-required"),
        };

        if self.chapter_id.is_some() == self.owner_id.is_some() {
            return Err(invalid_args());
        }

        if let Some(chapter_id) = self.chapter_id {
            return accept(AssignmentInfoListSpec::Chapter {
                chapter_id,
                role: self.role,
                incl_opt: self.incl_opt,
                offset: self.offset,
                limit: self.limit,
            });
        }

        accept(AssignmentInfoListSpec::User {
            owner_id: self.owner_id.ok_or_else(invalid_args)?,
            role: self.role,
            incl_opt: self.incl_opt,
            offset: self.offset,
            limit: self.limit,
        })
    }
}

/// Input parameters for updating assignment roles.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateAssignmentRolesParams {
    /// Target chapter identifier.
    pub chapter_id: String,
    /// Target user identifier.
    pub user_id: String,

    /// New role mask to apply.
    pub roles: RoleMask,
}

/// Input parameters for a user joining a chapter as a worker via role
/// selection.
///
/// The role mask must contain role bits that are valid for volunteer
/// assignment; the use case layer validates this before applying.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct JoinChapterAssignmentParams {
    /// Chapter to join as an assignee.
    pub chapter_id: String,

    /// Volunteer role mask.
    pub roles: RoleMask,
}
