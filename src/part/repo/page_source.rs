//! Repository trait for the page domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::page_source::{
    ApplyPageSourceManifest, DeletePageSources, GetPageSourceInfo,
    GetPageSourceInfoExcluded, GetPageSourceUnitScope,
    GetPageSourceUnitScopeExcluded, ListFirstPageSourceInfos,
    ListPageSourceInfos, ListPageSourceInfosExcluded,
    ListPageSourceRawIdentInfos, ListPageSourceUnitDiffStats,
    ListPageSourceUnitFlaggedStats, SetPageSourceUnitCountMetrics,
    ShiftPageSourceIndexesTemporary, UpdatePageSourceRawIdents,
};
use crate::result::BaseError;

/// source page repository operations over standalone runs and coordinated steps.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> ListPageSourceRawIdentInfos<'a>,
        for<'a> GetPageSourceInfo<'a>,
        for<'a> GetPageSourceUnitScope<'a>,
        for<'a> ListPageSourceInfos<'a>,
        for<'a> ListFirstPageSourceInfos<'a>,
        for<'a> ListPageSourceUnitDiffStats<'a>,
        for<'a> ListPageSourceUnitFlaggedStats<'a>,
    ),
    step(
        for<'a> UpdatePageSourceRawIdents<'a>,
        for<'a> GetPageSourceInfo<'a>,
        for<'a> GetPageSourceUnitScope<'a>,
        for<'a> ListPageSourceInfos<'a>,
        for<'a> ListPageSourceInfosExcluded<'a>,
        for<'a> GetPageSourceInfoExcluded<'a>,
        for<'a> GetPageSourceUnitScopeExcluded<'a>,
        for<'a> SetPageSourceUnitCountMetrics<'a>,
        for<'a> ShiftPageSourceIndexesTemporary<'a>,
        for<'a> ApplyPageSourceManifest<'a>,
        for<'a> DeletePageSources<'a>,
    ),
)]
pub trait PageSourceRepo<C> {}
