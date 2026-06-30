use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_chapter_assignee, check_user_is_chapter_translator_or_proofreader,
    check_user_is_team_member_by_chapter,
};
use crate::part::repo::step::assignment::GetInfoByChapterIdAndUserId;
use crate::part::repo::step::chapter::GetInfoById as ChapterGetInfoById;
use crate::part::repo::step::comic::GetInfoById as ComicGetInfoById;
use crate::part::repo::step::member::FindInfoByUserIdAndTeamId;
use crate::part::repo::step::workset::GetInfoById as WorksetGetInfoById;
use crate::part::shared::proxy::ProxyExecute;
use crate::result::{ExpectedVariant, RootError, RootResult};

/// Chapter import and export permission rules.
pub struct ChapterPortPermComplex;

impl ChapterPortPermComplex {
    /// Verify the caller may export chapter translations.
    pub async fn can_user_export<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ChapterGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>
            + for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RootError>,
    {
        let member_check = check_user_is_team_member_by_chapter(proxy, user_id, chapter_id).await;

        if member_check.is_ok() {
            return Ok(());
        }

        match check_user_is_chapter_assignee(proxy, user_id, chapter_id).await {
            Ok(()) => Ok(()),
            Err(RootError::Expected {
                variant: ExpectedVariant::PermDeny,
                ..
            }) => Err(chapter_port_export_permission_error()),
            Err(e) => Err(e),
        }
    }

    /// Verify the caller may import chapter translations.
    pub async fn can_user_import<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RootError>,
    {
        match check_user_is_chapter_translator_or_proofreader(proxy, user_id, chapter_id).await {
            Ok(()) => Ok(()),
            Err(RootError::Expected {
                variant: ExpectedVariant::PermDeny,
                ..
            }) => Err(chapter_port_import_permission_error()),
            Err(e) => Err(e),
        }
    }
}

fn chapter_port_export_permission_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::PermDeny,
        message: trl("error-chapter-port-export-permission-required"),
    }
}

fn chapter_port_import_permission_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::PermDeny,
        message: trl("error-chapter-port-import-permission-required"),
    }
}
