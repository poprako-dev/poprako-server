//! Repository operations consumed by versioned review editing.

use poprako_orchestra::Oper;

use crate::model::read::proj::issue::IssueInfo;
use crate::model::read::proj::issue_edit::{
    IssueEditReviewVer, IssueEditVariantCount,
};
use crate::model::write::issue_edit::{
    IssueEditPageRepl, IssueEditVariantRename,
};

/// Reads the chapter's review context in a consistent transaction.
#[derive(Oper)]
#[oper(output = IssueEditReviewVer)]
pub struct GetReviewVer<'a> {
    /// Owning chapter.
    pub chapter_id: &'a str,
}

/// Reads the persisted page revision, also requiring that page to exist.
#[derive(Oper)]
#[oper(output = i64)]
pub struct GetIssuePageRevision<'a> {
    /// Composite page identity.
    pub page_artwork_id: &'a str,
}

/// Reads annotations in page order.
#[derive(Oper)]
#[oper(output = Vec<IssueInfo>)]
pub struct ListPageIssueInfos<'a> {
    /// Composite page identity.
    pub page_artwork_id: &'a str,
}

/// Aggregates actual category references across all chapter pages.
#[derive(Oper)]
#[oper(output = Vec<IssueEditVariantCount>)]
pub struct ListIssueVariantCounts<'a> {
    /// Owning chapter.
    pub chapter_id: &'a str,
}

/// Replaces page contents and advances page and chapter versions atomically.
#[derive(Oper)]
#[oper(output = ())]
pub struct ReplacePageIssues<'a> {
    /// Validated replacement under the chapter lock.
    pub repl: &'a IssueEditPageRepl,
}

/// Renames chapter references and returns the identities of changed pages.
#[derive(Oper)]
#[oper(output = Vec<String>)]
pub struct RenameIssueVariants<'a> {
    /// Validated rename under the chapter lock.
    pub rename: &'a IssueEditVariantRename,
}

/// Finds existing IDs so the domain can reject cross-page identity reuse.
#[derive(Oper)]
#[oper(output = Vec<IssueInfo>)]
pub struct ListIssueInfosByIds<'a> {
    /// Identities in the submitted replacement.
    pub ids: &'a [&'a str],
}
