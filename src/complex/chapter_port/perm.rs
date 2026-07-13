use poprako_orchestra::Proxy;

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_chapter_assignee,
    check_user_is_chapter_translator_or_proofreader,
    check_user_is_team_member_by_chapter,
};
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfo;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::result::{ExpectedVariant, RegularError, RegularResult};

/// Chapter import and export permission rules.
pub struct ChapterPortPermComplex;

impl ChapterPortPermComplex {
    /// Verify the caller may export chapter translations.
    pub async fn ensure_user_can_export<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = RegularError>
            + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = RegularError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>
            + for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = RegularError>,
    {
        let member_check =
            check_user_is_team_member_by_chapter(proxy, user_id, chapter_id)
                .await;

        if member_check.is_ok() {
            return Ok(());
        }

        match check_user_is_chapter_assignee(proxy, user_id, chapter_id).await {
            //
            Ok(()) => Ok(()),

            Err(RegularError::Expected {
                variant: ExpectedVariant::Perm,
                ..
            }) => Err(chapter_port_export_permission_error()),

            Err(e) => Err(e),
        }
    }

    /// Verify the caller may import chapter translations.
    pub async fn ensure_user_can_import<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = RegularError>,
    {
        match check_user_is_chapter_translator_or_proofreader(
            proxy, user_id, chapter_id,
        )
        .await
        {
            Ok(()) => Ok(()),

            Err(RegularError::Expected {
                variant: ExpectedVariant::Perm,
                ..
            }) => Err(chapter_port_import_permission_error()),

            Err(e) => Err(e),
        }
    }
}

/// Construct a "chapter port export permission required" permission error.
fn chapter_port_export_permission_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-port-export-permission-required"),
    }
}

/// Construct a "chapter port import permission required" permission error.
fn chapter_port_import_permission_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-port-import-permission-required"),
    }
}
