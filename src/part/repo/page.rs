//! Repository trait for the page domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::page::{
    ClearPageImagesForPublish, CreatePages, DeletePages, GetPageInfo,
    GetPageInfoExcluded, ListFirstPageInfos, ListPageInfos,
    ListPageInfosExcluded, MarkPageImageUploaded, ReservePageImage,
    SetPageImageUploaded, SetPageUnitCounters, ShiftPageIndexesTemporary,
    UpdatePageManifest,
};
use crate::result::BaseError;

/// Page repository operations over standalone runs and coordinated steps.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> GetPageInfo<'a>,
        for<'a> ListPageInfos<'a>,
        for<'a> ListFirstPageInfos<'a>,
    ),
    step(
        for<'a> GetPageInfo<'a>,
        for<'a> ListPageInfos<'a>,
        for<'a> ListPageInfosExcluded<'a>,
        for<'a> CreatePages<'a>,
        for<'a> GetPageInfoExcluded<'a>,
        for<'a> ReservePageImage<'a>,
        for<'a> MarkPageImageUploaded<'a>,
        for<'a> SetPageImageUploaded<'a>,
        for<'a> SetPageUnitCounters<'a>,
        for<'a> ShiftPageIndexesTemporary<'a>,
        for<'a> UpdatePageManifest<'a>,
        for<'a> ClearPageImagesForPublish<'a>,
        for<'a> DeletePages<'a>,
    ),
)]
pub trait PageRepo<C> {}
