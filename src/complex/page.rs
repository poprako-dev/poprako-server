//! Complex-domain opers for page entities.

use poprako_orchestra::Proxy;

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_member,
};
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfo;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::role::RoleField;

/// Pure chapter-page manifest matching.
pub mod manifest;

/// Domain opers for page entities.
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
        format!(
            "page/chapter_{}/{}-{}.{}",
            chapter_id, page_id, image_version, file_ext
        )
    }
}

/// Permission-gate opers for page entities.
pub struct PagePermComplex;

impl PagePermComplex {
    /// Verify the caller may reserve page images for the chapter.
    pub async fn ensure_user_can_reserve<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
    {
        check_reserve_role(proxy, user_id, chapter_id).await
    }

    /// Verify the caller may list pages under a chapter.
    pub async fn ensure_user_can_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = BaseError>
            + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>
            + for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
    {
        let chapter_info = proxy
            .exec(&GetChapterInfo {
                id: chapter_id,
                incls: &[],
            })
            .await?;

        let comic_info = proxy
            .exec(&GetComicInfo {
                id: &chapter_info.comic_id,
                incls: &[],
            })
            .await?;

        let workset_info = proxy
            .exec(&GetWorksetInfo {
                id: &comic_info.workset_id,
            })
            .await?;

        let member_check =
            check_user_is_team_member(proxy, user_id, &workset_info.team_id)
                .await;

        if member_check.is_ok() {
            return accept(());
        }

        check_any_assignment(proxy, user_id, chapter_id).await
    }

    /// Verify the caller may confirm a page image upload.
    pub async fn ensure_user_can_mark_image_uploaded<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
    {
        check_upload_role(proxy, user_id, chapter_id).await
    }

    /// Verify the caller may delete all pages under the chapter.
    pub async fn ensure_user_can_delete<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = BaseError>
            + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let chapter_info = proxy
            .exec(&GetChapterInfo {
                id: chapter_id,
                incls: &[],
            })
            .await?;

        let comic_info = proxy
            .exec(&GetComicInfo {
                id: &chapter_info.comic_id,
                incls: &[],
            })
            .await?;

        let workset_info = proxy
            .exec(&GetWorksetInfo {
                id: &comic_info.workset_id,
            })
            .await?;

        check_user_is_team_admin(proxy, user_id, &workset_info.team_id).await
    }
}

// Return a permission error when page image reservation requires assignment.
fn page_reserve_role_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-page-reserve-role-required"),
    }
}

// Return a permission error when page image upload confirmation requires assignment.
fn page_upload_role_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-page-upload-role-required"),
    }
}

// Verify the caller is assigned as `RAW_PROVIDER` or `REVIEWER` on the
// chapter, which is required for page image reservation.
async fn check_reserve_role<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseResult<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
{
    let assignment_info = proxy
        .exec(&FindAssignmentInfo::ChapterUser {
            chapter_id,
            user_id,
        })
        .await?;

    let Some(assignment_info) = assignment_info else {
        return Err(page_reserve_role_err());
    };

    if !assignment_info
        .roles
        .has_any_role(&[RoleField::RAW_PROVIDER, RoleField::REVIEWER])
    {
        return Err(page_reserve_role_err());
    }

    accept(())
}

// Verify the caller is assigned as `RAW_PROVIDER` on the chapter, which
// is required for page image upload confirmation.
async fn check_upload_role<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseResult<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
{
    let assignment_info = proxy
        .exec(&FindAssignmentInfo::ChapterUser {
            chapter_id,
            user_id,
        })
        .await?;

    let Some(assignment_info) = assignment_info else {
        return Err(page_upload_role_err());
    };

    if !assignment_info
        .roles
        .has_any_role(&[RoleField::RAW_PROVIDER])
    {
        return Err(page_upload_role_err());
    }

    accept(())
}

// Verify the caller has any assignment on the chapter (any role qualifies).
async fn check_any_assignment<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseResult<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
{
    let assignment_info = proxy
        .exec(&FindAssignmentInfo::ChapterUser {
            chapter_id,
            user_id,
        })
        .await?;

    if assignment_info.is_none() {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-member-required"),
        });
    }

    accept(())
}
