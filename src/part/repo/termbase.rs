use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::termbase::{
    CreateTermbase, DeleteTermbase, LockTermbaseExcluded,
};
use crate::result::BaseError;

/// Termbase repository operations.
///
/// `CreateTermbase` runs independently, while the lock and delete operations
/// step through the context supplied by [`Nucl::coord`].
pub trait TermbaseRepo<C>:
    for<'a> Run<CreateTermbase<'a>, Error = BaseError>
    + for<'a> Step<LockTermbaseExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<DeleteTermbase<'a>, C, Error = BaseError>
{
}
