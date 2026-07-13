//! Repository trait for the page domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::page::{
    CreatePages, DeletePages, GetPageInfo, GetPageInfoExcluded, ListPageInfos,
    MarkPageImageUploaded, ReservePageImage, SetPageUnitCounters,
};
use crate::result::RegularError;

/// Page repository operations over standalone runs and coordinated steps.
pub trait PageRepo<C>:
    for<'a> Run<GetPageInfo<'a>, Error = RegularError>
    + for<'a> Run<ListPageInfos<'a>, Error = RegularError>
    + for<'a> Step<GetPageInfo<'a>, C, Error = RegularError>
    + for<'a> Step<ListPageInfos<'a>, C, Error = RegularError>
    + for<'a> Step<CreatePages<'a>, C, Error = RegularError>
    + for<'a> Step<GetPageInfoExcluded<'a>, C, Error = RegularError>
    + for<'a> Step<ReservePageImage<'a>, C, Error = RegularError>
    + for<'a> Step<MarkPageImageUploaded<'a>, C, Error = RegularError>
    + for<'a> Step<SetPageUnitCounters<'a>, C, Error = RegularError>
    + for<'a> Step<DeletePages<'a>, C, Error = RegularError>
{
}
