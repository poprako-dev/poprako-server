//! Production capabilities for current review issues.

use poprako_orchestra::Oper;

use crate::model::read::proj::issue::IssueInfo;
use crate::model::write::issue::ChapterIssuesRepl;

/// Reads a Chapter's issues ordered by review page and issue position.
#[derive(Oper)]
#[oper(output = Vec<IssueInfo>)]
pub struct ListIssueInfos<'a> {
    /// Chapter to read.
    pub chapter_id: &'a str,
}

/// Replaces a Chapter's entire current review under a Chapter lock.
#[derive(Oper)]
#[oper(output = ())]
pub struct ReplaceChapterIssues<'a> {
    /// Complete validated replacement.
    pub repl: &'a ChapterIssuesRepl<'a>,
}
