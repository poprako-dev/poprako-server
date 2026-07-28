use poprako_orchestra::drive;

use crate::part::repo::oper::workset::{
    AllocWorksetComicIndex, CreateWorkset, DeleteWorkset, GetWorksetInfo,
    GetWorksetInfoExcluded, ListWorksetInfos, ListWorksetInfosExcluded,
    UpdateWorkset, UpdateWorksetComicCount,
};
use crate::result::BaseError;

/// Workset repository operations.
///
/// Independent reads and updates use [`poprako_orchestra::Run`]. Transactional reads, mutations,
/// and pessimistic locks use [`poprako_orchestra::Step`] with the caller-coordinated context.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> GetWorksetInfo<'a>,
        for<'a> ListWorksetInfos<'a>,
        for<'a> UpdateWorkset<'a>,
    ),
    step(
        for<'a> GetWorksetInfo<'a>,
        for<'a> ListWorksetInfos<'a>,
        for<'a> GetWorksetInfoExcluded<'a>,
        for<'a> ListWorksetInfosExcluded<'a>,
        for<'a> CreateWorkset<'a>,
        for<'a> DeleteWorkset<'a>,
        for<'a> AllocWorksetComicIndex<'a>,
        for<'a> UpdateWorksetComicCount<'a>,
    ),
)]
pub trait WorksetRepo<C> {}
