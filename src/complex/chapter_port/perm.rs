use crate::complex::unit::UnitPermComplex;
use crate::part::repo::step::assignment::GetInfoByChapterIdAndUserId;
use crate::part::repo::step::chapter::GetInfoById as ChapterGetInfoById;
use crate::part::repo::step::comic::GetInfoById as ComicGetInfoById;
use crate::part::repo::step::member::FindInfoByUserIdAndTeamId;
use crate::part::repo::step::workset::GetInfoById as WorksetGetInfoById;
use crate::part::shared::proxy::ProxyExecute;
use crate::result::{RootError, RootResult};

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
        UnitPermComplex::can_user_list_infos(proxy, user_id, chapter_id).await
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
        UnitPermComplex::can_user_save_infos(proxy, user_id, chapter_id).await
    }
}
