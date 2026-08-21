use poprako_orchestra::drive;

use crate::part::repo::oper::term::{
    CreateTerm, DeleteTerm, DeleteTerms, GetTermInfo, GetTermInfoExcluded,
    ListTermInfos, LockTerm, UpdateTerm, UpsertTerms,
};
use crate::result::BaseError;

/// Terminology-entry repository operations.
///
/// Independent reads use [`poprako_orchestra::Run`]. Mutations and pessimistic reads use [`poprako_orchestra::Step`]
/// with the context coordinated by the caller.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> GetTermInfo<'a>,
        for<'a> ListTermInfos<'a>,
    ),
    step(
        for<'a> CreateTerm<'a>,
        for<'a> GetTermInfoExcluded<'a>,
        for<'a> ListTermInfos<'a>,
        for<'a> LockTerm<'a>,
        for<'a> UpdateTerm<'a>,
        for<'a> UpsertTerms<'a>,
        for<'a> DeleteTerm<'a>,
        for<'a> DeleteTerms<'a>,
    ),
)]
pub trait TermRepo<C> {}
