//! Complex-domain opers for page entities.

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_member,
};
use crate::part::repo::step::assignment::{
    AssignmentStep, GetInfoByChapterIdAndUserId,
};
use crate::part::repo::step::chapter::{
    ChapterStep, GetInfoById as ChapterGetInfoById,
};
use crate::part::repo::step::comic::{
    ComicStep, GetInfoById as ComicGetInfoById,
};
use crate::part::repo::step::member::FindInfoByUserIdAndTeamId;
use crate::part::repo::step::workset::{
    GetInfoById as WorksetGetInfoById, WorksetStep,
};
use crate::part::shared::proxy::ProxyExecute;
use crate::result::{ExpectedVariant, RegularError, RegularResult, accept};
use crate::util::next_snowflake_id;
use crate::value::role::RoleField;

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
        image_version: i64,
        file_ext: &str,
    ) -> String {
        format!(
            "chapter_{}/page_{}-{}.{}",
            chapter_id, page_id, image_version, file_ext
        )
    }
}

/// Permission-gate opers for page entities.
pub struct PagePermComplex;

impl PagePermComplex {
    /// Verify the caller may reserve page images for the chapter.
    pub async fn can_user_reserve<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<
                GetInfoByChapterIdAndUserId<'a>,
                Error = RegularError,
            >,
    {
        check_reserve_role(proxy, user_id, chapter_id).await
    }

    /// Verify the caller may list pages under a chapter.
    pub async fn can_user_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<ChapterGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<
                FindInfoByUserIdAndTeamId<'a>,
                Error = RegularError,
            > + for<'a> ProxyExecute<
                GetInfoByChapterIdAndUserId<'a>,
                Error = RegularError,
            >,
    {
        let chapter_info = proxy
            .execute(&ChapterStep::get_info_by_id(chapter_id, &[]))
            .await?;

        let comic_info = proxy
            .execute(&ComicStep::get_info_by_id(&chapter_info.comic_id, &[]))
            .await?;

        let workset_info = proxy
            .execute(&WorksetStep::get_info_by_id(&comic_info.workset_id))
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
    pub async fn can_user_mark_image_uploaded<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<
                GetInfoByChapterIdAndUserId<'a>,
                Error = RegularError,
            >,
    {
        check_upload_role(proxy, user_id, chapter_id).await
    }

    /// Verify the caller may delete all pages under the chapter.
    pub async fn can_user_delete<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<ChapterGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<
                FindInfoByUserIdAndTeamId<'a>,
                Error = RegularError,
            >,
    {
        let chapter_info = proxy
            .execute(&ChapterStep::get_info_by_id(chapter_id, &[]))
            .await?;

        let comic_info = proxy
            .execute(&ComicStep::get_info_by_id(&chapter_info.comic_id, &[]))
            .await?;

        let workset_info = proxy
            .execute(&WorksetStep::get_info_by_id(&comic_info.workset_id))
            .await?;

        check_user_is_team_admin(proxy, user_id, &workset_info.team_id).await
    }
}

async fn check_reserve_role<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> RegularResult<()>
where
    P: for<'a> ProxyExecute<
            GetInfoByChapterIdAndUserId<'a>,
            Error = RegularError,
        >,
{
    let assignment_info = proxy
        .execute(&AssignmentStep::get_info_by_chapter_id_and_user_id(
            chapter_id, user_id,
        ))
        .await?;

    let Some(assignment_info) = assignment_info else {
        return Err(page_reserve_role_error());
    };

    if !assignment_info
        .roles
        .has_any_role(&[RoleField::RAW_PROVIDER, RoleField::REVIEWER])
    {
        return Err(page_reserve_role_error());
    }

    accept(())
}

async fn check_upload_role<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> RegularResult<()>
where
    P: for<'a> ProxyExecute<
            GetInfoByChapterIdAndUserId<'a>,
            Error = RegularError,
        >,
{
    let assignment_info = proxy
        .execute(&AssignmentStep::get_info_by_chapter_id_and_user_id(
            chapter_id, user_id,
        ))
        .await?;

    let Some(assignment_info) = assignment_info else {
        return Err(page_upload_role_error());
    };

    if !assignment_info
        .roles
        .has_any_role(&[RoleField::RAW_PROVIDER])
    {
        return Err(page_upload_role_error());
    }

    accept(())
}

async fn check_any_assignment<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> RegularResult<()>
where
    P: for<'a> ProxyExecute<
            GetInfoByChapterIdAndUserId<'a>,
            Error = RegularError,
        >,
{
    let assignment_info = proxy
        .execute(&AssignmentStep::get_info_by_chapter_id_and_user_id(
            chapter_id, user_id,
        ))
        .await?;

    if assignment_info.is_none() {
        return Err(RegularError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-member-required"),
        });
    }

    accept(())
}

fn page_reserve_role_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-page-reserve-role-required"),
    }
}

fn page_upload_role_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-page-upload-role-required"),
    }
}
