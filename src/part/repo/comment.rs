//! Repository traits for the comment domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::comment::{CreateComment, ListCommentInfos};
use crate::result::RegularError;

/// Comment repository operations.
///
/// Independent lists use [`Run`], while creation steps through the context
/// coordinated by the caller.
pub trait CommentRepo<C>:
    for<'a> Run<ListCommentInfos<'a>, Error = RegularError>
    + for<'a> Step<CreateComment<'a>, C, Error = RegularError>
{
}
