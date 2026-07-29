use poprako_orchestra::Oper;

use crate::model::comment::{CommentEntry, CommentInfo, CommentListSpec};

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
