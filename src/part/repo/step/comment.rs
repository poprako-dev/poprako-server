//! Step types for comment repository opers.

use poprako_transactional::step::Step;

use crate::model::comment::{CommentForm, CommentInfo, CommentListSpec};

/// Step that lists comments by query specification.
pub struct ListInfos<'a> {
    pub spec: &'a CommentListSpec,
}

impl<'a> Step for ListInfos<'a> {
    type Output = Vec<CommentInfo>;
}

/// Step that inserts a new comment row.
pub struct Create<'a> {
    pub form: &'a CommentForm,
}

impl<'a> Step for Create<'a> {
    type Output = CommentInfo;
}

/// Factory for constructing comment repository [`Step`] values.
pub struct CommentStep;

impl CommentStep {
    /// Constructs a step to list comments.
    pub fn list_infos<'a>(spec: &'a CommentListSpec) -> ListInfos<'a> {
        ListInfos { spec }
    }

    /// Constructs a step to insert a new comment.
    pub fn create<'a>(form: &'a CommentForm) -> Create<'a> {
        Create { form }
    }
}
