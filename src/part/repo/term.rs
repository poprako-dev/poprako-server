use poprako_orchestra::Step;

use crate::part::repo::oper::term::DeleteTerms;
use crate::result::RegularError;

/// Term repository operations that run within a caller-owned transaction.
pub trait TermRepo<C>:
    for<'v, 'a> Step<DeleteTerms<'v, 'a>, C, Error = RegularError>
{
}
