use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::workset::{
    AllocWorksetComicIndex, CreateWorkset, DeleteWorkset, GetWorksetInfo,
    GetWorksetInfoExcluded, ListWorksetInfos, ListWorksetInfosExcluded,
    UpdateWorkset, UpdateWorksetComicCount,
};
use crate::result::BaseError;

/// Workset repository operations.
///
/// Independent reads and updates use [`Run`]. Transactional reads, mutations,
/// and pessimistic locks use [`Step`] with the caller-coordinated context.
pub trait WorksetRepo<C>:
    for<'a> Run<GetWorksetInfo<'a>, Error = BaseError>
    + for<'a> Run<ListWorksetInfos<'a>, Error = BaseError>
    + for<'a> Run<UpdateWorkset<'a>, Error = BaseError>
    + for<'a> Step<GetWorksetInfo<'a>, C, Error = BaseError>
    + for<'a> Step<ListWorksetInfos<'a>, C, Error = BaseError>
    + for<'a> Step<GetWorksetInfoExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<ListWorksetInfosExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<CreateWorkset<'a>, C, Error = BaseError>
    + for<'a> Step<DeleteWorkset<'a>, C, Error = BaseError>
    + for<'a> Step<AllocWorksetComicIndex<'a>, C, Error = BaseError>
    + for<'a> Step<UpdateWorksetComicCount<'a>, C, Error = BaseError>
{
}

impl<T, C> WorksetRepo<C> for T
where
    T: for<'a> Run<GetWorksetInfo<'a>, Error = BaseError>
       + for<'a> Run<ListWorksetInfos<'a>, Error = BaseError>
       + for<'a> Run<UpdateWorkset<'a>, Error = BaseError>
       + for<'a> Step<GetWorksetInfo<'a>, C, Error = BaseError>
       + for<'a> Step<ListWorksetInfos<'a>, C, Error = BaseError>
       + for<'a> Step<GetWorksetInfoExcluded<'a>, C, Error = BaseError>
       + for<'a> Step<ListWorksetInfosExcluded<'a>, C, Error = BaseError>
       + for<'a> Step<CreateWorkset<'a>, C, Error = BaseError>
       + for<'a> Step<DeleteWorkset<'a>, C, Error = BaseError>
       + for<'a> Step<AllocWorksetComicIndex<'a>, C, Error = BaseError>
       + for<'a> Step<UpdateWorksetComicCount<'a>, C, Error = BaseError>,
{
}
