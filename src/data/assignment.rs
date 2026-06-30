//! Data transfer objects for assignment use cases.

use serde::Deserialize;

use poprako_macro::Paginate;
use poprako_util::i18n::trl;
use poprako_util::time::ToUnixMilli;

use crate::data::user::UserInfoVal;
use crate::model::assignment::{AssignmentInfo, AssignmentListSpec};
use crate::model::role::{RoleField, RoleMask};
use crate::part::image::ImagePool;
use crate::result::{ExpectedVariant, RootError, RootResult};
use crate::value::assignment::AssignmentInclOpt;

/// Presentation-ready chapter assignment information.
pub struct AssignmentInfoVal {
    pub id: String,

    pub chapter_id: String,
    pub user_id: String,

    pub user: Option<UserInfoVal>,

    pub roles: RoleMask,

    pub created_at: i64,
    pub updated_at: i64,
}

impl From<AssignmentInfo> for AssignmentInfoVal {
    fn from(model: AssignmentInfo) -> Self {
        Self {
            id: model.id,
            chapter_id: model.chapter_id,
            user_id: model.user_id,
            user: None,
            roles: model.roles,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}

impl AssignmentInfoVal {
    /// Converts an assignment model into a presentation-ready value,
    /// resolving included user avatar when present.
    pub async fn from_model<P>(image_pool: &P, model: AssignmentInfo) -> RootResult<Self>
    where
        P: ImagePool,
    {
        let user = match model.user {
            Some(user_info) => Some(UserInfoVal::from_model(image_pool, user_info).await?),
            None => None,
        };

        Ok(Self {
            id: model.id,
            chapter_id: model.chapter_id,
            user_id: model.user_id,
            user,
            roles: model.roles,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        })
    }
}

/// Input parameters for listing assignments by chapter or owner user.
#[Paginate]
#[derive(Deserialize)]
pub struct ListAssignmentInfosData {
    pub chapter_id: Option<String>,
    pub owner_id: Option<String>,

    pub role: Option<RoleField>,

    #[serde(default)]
    pub incl_opt: Vec<AssignmentInclOpt>,
}

impl TryInto<AssignmentListSpec> for ListAssignmentInfosData {
    type Error = RootError;

    fn try_into(self) -> RootResult<AssignmentListSpec> {
        let invalid_args_err = || RootError::Expected {
            variant: ExpectedVariant::ArgsInvalid,
            message: trl("error-chapter-or-user-required"),
        };

        if self.chapter_id.is_some() == self.owner_id.is_some() {
            return Err(invalid_args_err());
        }

        if let Some(chapter_id) = self.chapter_id {
            return Ok(AssignmentListSpec::Chapter {
                chapter_id,
                role: self.role,
                incl_opt: self.incl_opt,
                offset: self.offset,
                limit: self.limit,
            });
        }

        Ok(AssignmentListSpec::User {
            owner_id: self.owner_id.ok_or_else(invalid_args_err)?,
            role: self.role,
            incl_opt: self.incl_opt,
            offset: self.offset,
            limit: self.limit,
        })
    }
}

/// Input parameters for updating assignment roles.
pub struct UpdateAssignmentRoleData {
    pub chapter_id: String,
    pub user_id: String,

    pub roles: RoleMask,
}

/// Input parameters for a user joining a chapter as a worker via role
/// selection.
///
/// The role mask must contain role bits that are valid for volunteer
/// assignment; the use case layer validates this before applying.
pub struct JoinChapterData {
    pub chapter_id: String,

    pub roles: RoleMask,
}
