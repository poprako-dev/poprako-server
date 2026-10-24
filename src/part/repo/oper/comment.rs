use poprako_orchestra::Oper;

use crate::model::read::proj::comment::CommentInfo;
use crate::model::read::spec::comment::CommentListSpec;
use crate::model::write::comment::CommentEntry;

/// Lists comment infos selected by a query specification.
#[derive(Oper)]
#[oper(output = Vec<CommentInfo>)]
pub struct ListCommentInfos<'a> {
    /// The filter and pagination specification.
    pub spec: &'a CommentListSpec,
}

/// Creates a comment.
#[derive(Oper)]
#[oper(output = CommentInfo)]
pub struct CreateComment<'a> {
    /// The comment entry to insert.
    pub entry: &'a CommentEntry,
}
