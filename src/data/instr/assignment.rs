//! Instr DTOs for the assignment domain.

//! Data transfer objects for assignment use cases.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use poprako_util::i18n::trl;

use crate::model::read::spec::assignment::AssignmentListSpec;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::assignment::AssignmentInclOpt;
use crate::value::role::{RoleField, RoleMask};

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
pub struct ListAssignmentInfosInstr {
    //
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

impl TryInto<AssignmentListSpec> for ListAssignmentInfosInstr {
    // Validate exclusive list mode parameters before converting to domain spec.
    type Error = BaseError;

    // Convert validated query parameters into the domain list spec.
    fn try_into(self) -> BaseRest<AssignmentListSpec> {
        //
        let Self {
            chapter_id,
            owner_id,
            role,
            incl_opt,
            offset,
            limit,
        } = self;

        if chapter_id.is_some() == owner_id.is_some() {
            //
            let err_message = trl("error-chapter-or-user-required");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                chapter_id = ?chapter_id,
                owner_id = ?owner_id,
                role = ?role,
                "expected error: assignment list requires one scope identifier",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        if let Some(chapter_id) = chapter_id {
            return accept(AssignmentListSpec::Chapter {
                chapter_id,
                role,
                incl_opt,
                offset,
                limit,
            });
        }

        let Some(owner_id) = owner_id else {
            //
            let err_message = trl("error-chapter-or-user-required");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                chapter_id = ?chapter_id,
                owner_id = ?owner_id,
                role = ?role,
                "expected error: assignment list owner scope is missing",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        };

        accept(AssignmentListSpec::User {
            owner_id,
            role,
            incl_opt,
            offset,
            limit,
        })
    }
}

/// Input parameters for updating assignment roles.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateAssignmentRolesInstr {
    //
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
pub struct JoinChapterAssignmentInstr {
    //
    /// Chapter to join as an assignee.
    pub chapter_id: String,

    /// Volunteer role mask.
    pub roles: RoleMask,
}
