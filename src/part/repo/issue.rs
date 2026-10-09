//! Repository capabilities for the chapter's single current review.

use poprako_orchestra::drive;

use crate::part::repo::oper::issue::{
    ClearChapterIssues, ListIssueInfos, ReplaceChapterIssues,
};
use crate::result::BaseError;

/// Read-only Page access and transactional chapter replacement or cleanup.
#[drive(
    context = C,
    error = BaseError,
    run(for<'a> ListIssueInfos<'a>),
    step(for<'a> ReplaceChapterIssues<'a>, for<'a> ClearChapterIssues<'a>),
)]
pub trait IssueRepo<C> {}
