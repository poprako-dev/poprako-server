use poprako_orchestra::drive;

use crate::part::repo::oper::termbase::{
    CreateTermbase, DeleteTermbase, GetTermbaseInfo, GetTermbaseInfoExcluded,
    ListTermbaseInfos, ListTermbaseInfosExcluded, TouchTermbase,
    UpdateTermbase, UpdateTermbaseTermCount,
};
use crate::result::BaseError;

/// Terminology-base repository operations.
///
/// Independent reads use [`poprako_orchestra::Run`]. Mutations and pessimistic reads use [`poprako_orchestra::Step`]
/// with the context coordinated by the caller.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> GetTermbaseInfo<'a>,
        for<'a> ListTermbaseInfos<'a>,
    ),
    step(
        for<'a> CreateTermbase<'a>,
        for<'a> GetTermbaseInfo<'a>,
        for<'a> GetTermbaseInfoExcluded<'a>,
        for<'a> ListTermbaseInfosExcluded<'a>,
        for<'a> UpdateTermbase<'a>,
        for<'a> UpdateTermbaseTermCount<'a>,
        for<'a> TouchTermbase<'a>,
        for<'a> DeleteTermbase<'a>,
    ),
)]
pub trait TermbaseRepo<C> {}
