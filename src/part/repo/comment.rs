//! Repository traits for the comment domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::comment::{CreateComment, ListCommentInfos};
use crate::result::BaseError;

/// Comment repository operations.
///
/// Comment operations execute independently through [`poprako_orchestra::Run`].
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> ListCommentInfos<'a>,
        for<'a> CreateComment<'a>,
    ),
)]
pub trait CommentRepo<C> {}
