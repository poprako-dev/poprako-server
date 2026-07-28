//! Repository traits for the comment domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::comment::{CreateComment, ListCommentInfos};
use crate::result::BaseError;

/// Comment repository operations.
///
/// Independent lists use [`poprako_orchestra::Run`], while creation steps through the context
/// coordinated by the caller.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> ListCommentInfos<'a>,
    ),
    step(
        for<'a> CreateComment<'a>,
    ),
)]
pub trait CommentRepo<C> {}
