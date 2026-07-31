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
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Chapter import and export perm rules.
pub struct ChapterPortPermComplex;

impl ChapterPortPermComplex {
    /// Verify the caller may export chapter translations.
    pub async fn ensure_user_can_export<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = BaseError>
            + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>
            + for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
    {
        let member_check =
            check_user_is_team_member_by_chapter(proxy, user_id, chapter_id)
                .await;

        if member_check.is_ok() {
            return accept(());
        }

        match check_user_is_chapter_assignee(proxy, user_id, chapter_id).await {
            //
            Ok(()) => accept(()),

            Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                ..
            }) => {
                //
                let err_message =
                    trl("error-chapter-port-export-perm-required");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Perm,
                    err_message = %err_message,
                    user_id = %user_id,
                    chapter_id = %chapter_id,
                    operation = "export",
                    "expected error: chapter port export perm required",
                );

                Err(BaseError::Expected {
                    variant: ExpectedVariant::Perm,
                    message: err_message,
                })
            }

            Err(e) => Err(e),
        }
    }

    /// Verify the caller may import chapter translations.
    pub async fn ensure_user_can_import<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
    {
        match check_user_is_chapter_translator_or_proofreader(
            proxy, user_id, chapter_id,
        )
        .await
        {
            Ok(()) => accept(()),

            Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                ..
            }) => {
                //
                let err_message =
                    trl("error-chapter-port-import-perm-required");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Perm,
                    err_message = %err_message,
                    user_id = %user_id,
                    chapter_id = %chapter_id,
                    operation = "import",
                    "expected error: chapter port import perm required",
                );

                Err(BaseError::Expected {
                    variant: ExpectedVariant::Perm,
                    message: err_message,
                })
            }

            Err(e) => Err(e),
        }
    }
}
