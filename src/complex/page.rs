//! Complex-domain opers for page entities.

use poprako_orchestra::{OperProxy as _, Proxy};

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_member,
};
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::team::ResolveTeamId;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
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
        //
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
    ) -> BaseRest<()>
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
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>
            + for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
    {
        let team_id = ResolveTeamId::Chapter { id: chapter_id }
            .proxy_on(proxy)
            .await?;

        let member_check =
            check_user_is_team_member(proxy, user_id, &team_id).await;

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
    ) -> BaseRest<()>
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
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let team_id = ResolveTeamId::Chapter { id: chapter_id }
            .proxy_on(proxy)
            .await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }
}

// Verify the caller is assigned as `RAW_PROVIDER` or `REVIEWER` on the
// chapter, which is required for page image reservation.
async fn check_reserve_role<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseRest<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id,
    }
    .proxy_on(proxy)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        let err_message = trl("error-page-reserve-role-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            "expected error: page reservation assignment missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    if !assignment_info
        .roles
        .has_any_role(&[RoleField::RAW_PROVIDER, RoleField::REVIEWER])
    {
        let err_message = trl("error-page-reserve-role-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            assignment_roles = ?assignment_info.roles,
            "expected error: page reservation role missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}

// Verify the caller is assigned as `RAW_PROVIDER` on the chapter, which
// is required for page image upload confirmation.
async fn check_upload_role<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseRest<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id,
    }
    .proxy_on(proxy)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        let err_message = trl("error-page-upload-role-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            "expected error: page upload assignment missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    if !assignment_info
        .roles
        .has_any_role(&[RoleField::RAW_PROVIDER])
    {
        let err_message = trl("error-page-upload-role-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            assignment_roles = ?assignment_info.roles,
            "expected error: page upload role missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}

// Verify the caller has any assignment on the chapter (any role qualifies).
async fn check_any_assignment<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseRest<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id,
    }
    .proxy_on(proxy)
    .await?;

    if assignment_info.is_none() {
        //
        let err_message = trl("error-team-member-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            "expected error: page assignment required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}
