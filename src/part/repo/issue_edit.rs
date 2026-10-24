//! Transactional versioned review capabilities.

use poprako_orchestra::drive;

use crate::part::repo::oper::issue_edit::{
    GetIssuePageRevision, GetReviewVer, ListIssueInfosByIds,
    ListIssueVariantCounts, ListPageIssueInfos, RenameIssueVariants,
    ReplacePageIssues,
};
use crate::result::BaseError;

/// Consistent review reads and coordinated writes.
#[drive(context = C, error = BaseError, step(
    for<'a> GetReviewVer<'a>,
    for<'a> GetIssuePageRevision<'a>,
    for<'a> ListPageIssueInfos<'a>,
    for<'a> ListIssueVariantCounts<'a>,
    for<'a> ReplacePageIssues<'a>,
    for<'a> RenameIssueVariants<'a>,
    for<'a> ListIssueInfosByIds<'a>,
))]
pub trait IssueEditRepo<C> {}
