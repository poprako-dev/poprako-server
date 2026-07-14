use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::workset::{
    AllocWorksetComicIndex, CreateWorkset, DeleteWorkset, GetWorksetInfo,
    GetWorksetInfoExcluded, ListWorksetInfos, ListWorksetInfosExcluded,
    UpdateWorkset, UpdateWorksetComicCount,
};
use crate::result::RegularError;

/// Workset repository operations.
///
/// Independent reads and updates use [`Run`]. Transactional reads, mutations,
/// and pessimistic locks use [`Step`] with the caller-coordinated context.
pub trait WorksetRepo<C>:
    for<'a> Run<GetWorksetInfo<'a>, Error = RegularError>
    + for<'a> Run<ListWorksetInfos<'a>, Error = RegularError>
    + for<'a> Run<UpdateWorkset<'a>, Error = RegularError>
    + for<'a> Step<GetWorksetInfo<'a>, C, Error = RegularError>
    + for<'a> Step<ListWorksetInfos<'a>, C, Error = RegularError>
    + for<'a> Step<GetWorksetInfoExcluded<'a>, C, Error = RegularError>
    + for<'a> Step<ListWorksetInfosExcluded<'a>, C, Error = RegularError>
    + for<'a> Step<CreateWorkset<'a>, C, Error = RegularError>
    + for<'a> Step<DeleteWorkset<'a>, C, Error = RegularError>
    + for<'a> Step<AllocWorksetComicIndex<'a>, C, Error = RegularError>
    + for<'a> Step<UpdateWorksetComicCount<'a>, C, Error = RegularError>
{
}
