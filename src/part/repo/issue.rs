//! Repository capabilities for the chapter's single current review.

use poprako_orchestra::drive;

use crate::part::repo::oper::issue::{ListIssueInfos, ReplaceChapterIssues};
use crate::result::BaseError;

/// Ordered Issue reads and transactional whole-Chapter review replacement.
#[drive(
    context = C,
    error = BaseError,
    run(for<'a> ListIssueInfos<'a>),
    step(for<'a> ReplaceChapterIssues<'a>),
)]
pub trait IssueRepo<C> {}
