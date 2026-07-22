//! Repository traits for the comment domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::comment::{CreateComment, ListCommentInfos};
use crate::result::BaseError;

/// Comment repository operations.
///
/// Independent lists use [`Run`], while creation steps through the context
/// coordinated by the caller.
pub trait CommentRepo<C>:
    for<'a> Run<ListCommentInfos<'a>, Error = BaseError>
    + for<'a> Step<CreateComment<'a>, C, Error = BaseError>
{
}

impl<T, C> CommentRepo<C> for T
where
    T: for<'a> Run<ListCommentInfos<'a>, Error = BaseError>
       + for<'a> Step<CreateComment<'a>, C, Error = BaseError>,
{
}
