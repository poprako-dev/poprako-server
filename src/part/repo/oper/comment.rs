use poprako_orchestra::Oper;

use crate::model::comment::{CommentEntry, CommentInfo, CommentListSpec};

/// Lists comment infos selected by a query specification.
pub struct ListCommentInfos<'a> {
    pub spec: &'a CommentListSpec,
}

impl Oper for ListCommentInfos<'_> {
    type Output = Vec<CommentInfo>;
}

/// Creates a comment.
pub struct CreateComment<'a> {
    pub entry: &'a CommentEntry,
}

impl Oper for CreateComment<'_> {
    type Output = CommentInfo;
}
