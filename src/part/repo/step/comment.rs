//! Step types for comment repository opers.

use poprako_transactional::step::Step;

use crate::model::comment_model;

/// Step that lists comments by query specification.
pub struct ListInfos<'a> {
    pub spec: &'a comment_model::ListSpec,
}

impl<'a> Step for ListInfos<'a> {
    type Output = Vec<comment_model::Info>;
}

/// Step that inserts a new comment row.
pub struct Create<'a> {
    pub form: &'a comment_model::Form,
}

impl<'a> Step for Create<'a> {
    type Output = comment_model::Info;
}

/// Factory for constructing comment repository [`Step`] values.
pub struct CommentStep;

impl CommentStep {
    /// Constructs a step to list comments.
    pub fn list_infos<'a>(spec: &'a comment_model::ListSpec) -> ListInfos<'a> {
        ListInfos { spec }
    }

    /// Constructs a step to insert a new comment.
    pub fn create<'a>(form: &'a comment_model::Form) -> Create<'a> {
        Create { form }
    }
}
