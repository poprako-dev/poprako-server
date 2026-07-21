//! Repository trait for the page domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::page::{
    ClearPageImagesForPublish, CreatePages, DeletePages, GetPageInfo,
    GetPageInfoExcluded, ListFirstPageInfos, ListPageInfos,
    ListPageInfosExcluded, MarkPageImageUploaded, ReservePageImage,
    SetPageUnitCounters, ShiftPageIndexesTemporary, UpdatePageManifest,
};
use crate::result::BaseError;

/// Page repository operations over standalone runs and coordinated steps.
pub trait PageRepo<C>:
    for<'a> Run<GetPageInfo<'a>, Error = BaseError>
    + for<'a> Run<ListPageInfos<'a>, Error = BaseError>
    + for<'a> Run<ListFirstPageInfos<'a>, Error = BaseError>
    + for<'a> Step<GetPageInfo<'a>, C, Error = BaseError>
    + for<'a> Step<ListPageInfos<'a>, C, Error = BaseError>
    + for<'a> Step<ListPageInfosExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<CreatePages<'a>, C, Error = BaseError>
    + for<'a> Step<GetPageInfoExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<ReservePageImage<'a>, C, Error = BaseError>
    + for<'a> Step<MarkPageImageUploaded<'a>, C, Error = BaseError>
    + for<'a> Step<SetPageUnitCounters<'a>, C, Error = BaseError>
    + for<'a> Step<ShiftPageIndexesTemporary<'a>, C, Error = BaseError>
    + for<'a> Step<UpdatePageManifest<'a>, C, Error = BaseError>
    + for<'a> Step<ClearPageImagesForPublish<'a>, C, Error = BaseError>
    + for<'a> Step<DeletePages<'a>, C, Error = BaseError>
{
}
