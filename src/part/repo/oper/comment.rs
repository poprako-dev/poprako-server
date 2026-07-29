use poprako_orchestra::Oper;

use crate::model::comment::{CommentEntry, CommentInfo, CommentListSpec};

/// Lists comment infos selected by a query specification.
pub struct ListCommentInfos<'a> {
    /// The filter and pagination specification.
    pub spec: &'a CommentListSpec,
}

impl Oper for ListCommentInfos<'_> {
    // List of matching comment infos.
    type Output = Vec<CommentInfo>;
}

/// Creates a comment.
pub struct CreateComment<'a> {
    /// The comment entry to insert.
    pub entry: &'a CommentEntry,
}

impl Oper for CreateComment<'_> {
    // The created comment info.
    type Output = CommentInfo;
}
