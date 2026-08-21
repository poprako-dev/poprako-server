//! Pure rules for page entities.

/// Pure chapter-page manifest matching.
pub mod manifest;

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_chapter_assignee, check_user_is_team_admin,
    check_user_is_team_member,
};
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::role::RoleField;

// Build and log one expected page-role permission error.
fn reject_role(message_key: &str, event: &'static str) -> BaseRest<()> {
    //
    let err_message = trl(message_key);

    tracing::warn!(
        err_variant = ?ExpectedVariant::Perm,
        err_message = %err_message,
        event,
        "expected page permission error",
    );

    Err(BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: err_message,
    })
}

// Verify that assignment evidence permits reserving page images.
fn check_reserve_role(assignment_info: &AssignmentInfo) -> BaseRest<()> {
    //
    if !assignment_info
        .roles
        .has_any_role(&[RoleField::RAW_PROVIDER, RoleField::REVIEWER])
    {
        return reject_role(
            "error-page-reserve-role-required",
            "page_reservation_role_missing",
        );
    }

    accept(())
}

// Verify that assignment evidence permits uploading page images.
fn check_upload_role(assignment_info: &AssignmentInfo) -> BaseRest<()> {
    //
    if !assignment_info
        .roles
        .has_any_role(&[RoleField::RAW_PROVIDER])
    {
        return reject_role(
            "error-page-upload-role-required",
            "page_upload_role_missing",
        );
    }

    accept(())
}

/// Pure domain operations for page entities.
pub struct PageComplex;

impl PageComplex {
    /// Generate a unique page identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Construct the object-storage key for a page image.
    pub fn gen_image_key(
        chapter_id: &str,
        page_id: &str,
        image_version: u32,
        file_ext: &str,
    ) -> String {
        //
        format!(
            "page/chapter_{}/{}-{}.{}",
            chapter_id, page_id, image_version, file_ext
        )
    }
}

/// Evidence that grants page-list access.
pub enum PageListAccess<'a> {
    //
    /// Access through team membership.
    Member {
        /// Team membership used to establish access.
        member_info: &'a MemberInfo,
    },

    /// Access through an assignment on the chapter.
    Assignee {
        /// Chapter assignment used to establish access.
        assignment_info: &'a AssignmentInfo,
    },
}

/// Pure permission rules for page entities.
pub struct PagePermComplex;

impl PagePermComplex {
    /// Verify the caller may reserve page images for the chapter.
    pub fn ensure_user_can_reserve(
        assignment_info: &AssignmentInfo,
    ) -> BaseRest<()> {
        check_reserve_role(assignment_info)
    }

    /// Verify the caller may list pages under a chapter.
    pub fn ensure_user_can_list_infos(
        access: PageListAccess<'_>,
    ) -> BaseRest<()> {
        //
        match access {
            //
            PageListAccess::Member { member_info } => {
                check_user_is_team_member(member_info)
            }

            PageListAccess::Assignee { assignment_info } => {
                check_user_is_chapter_assignee(assignment_info)
            }
        }
    }

    /// Verify the caller may confirm a page image upload.
    pub fn ensure_user_can_mark_image_uploaded(
        assignment_info: &AssignmentInfo,
    ) -> BaseRest<()> {
        check_upload_role(assignment_info)
    }

    /// Verify the caller may delete all pages under the chapter.
    pub fn ensure_user_can_delete(member_info: &MemberInfo) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }
}
