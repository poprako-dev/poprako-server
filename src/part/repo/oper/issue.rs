//! Production capabilities for current review issues.

use poprako_orchestra::Oper;

use crate::model::read::proj::issue::IssueInfo;
use crate::model::write::issue::ChapterIssuesRepl;

/// Reads one Page's issues in ascending index order.
#[derive(Oper)]
#[oper(output = Vec<IssueInfo>)]
pub struct ListIssueInfos<'a> {
    /// Page to read.
    pub page_id: &'a str,
}

/// Replaces a Chapter's entire current review under Chapter and Page locks.
#[derive(Oper)]
#[oper(output = ())]
pub struct ReplaceChapterIssues<'a> {
    /// Complete validated replacement.
    pub repl: &'a ChapterIssuesRepl<'a>,
}

/// Clears issues when a new artwork generation is first confirmed.
#[derive(Oper)]
#[oper(output = ())]
pub struct ClearChapterIssues<'a> {
    /// Locked owning Chapter.
    pub chapter_id: &'a str,
}
