//! Repository trait for the page domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::page::{
    ApplyPageManifest, DeletePages, GetPageInfo, GetPageInfoExcluded,
    ListEdittedDiffPageIds, ListFirstPageInfos, ListPageInfos,
    ListPageInfosExcluded, SetPageUnitCounters, ShiftPageIndexesTemporary,
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
        for<'a> ListEdittedDiffPageIds<'a>,
    ),
    step(
        for<'a> GetPageInfo<'a>,
        for<'a> ListPageInfos<'a>,
        for<'a> ListPageInfosExcluded<'a>,
        for<'a> GetPageInfoExcluded<'a>,
        for<'a> SetPageUnitCounters<'a>,
        for<'a> ShiftPageIndexesTemporary<'a>,
        for<'a> ApplyPageManifest<'a>,
        for<'a> DeletePages<'a>,
    ),
)]
pub trait PageRepo<C> {}
