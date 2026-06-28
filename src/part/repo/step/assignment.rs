//! Step types for assignment repository operations.

use poprako_transactional::step::Step;

use crate::model::assignment::{AssignmentForm, AssignmentInfo, AssignmentRoleUpdate};

/// Step that finds one assignment by chapter and user.
pub struct GetInfoByChapterUserId<'a> {
    pub chapter_id: &'a str,
    pub user_id: &'a str,
}

impl<'a> Step for GetInfoByChapterUserId<'a> {
    type Output = Option<AssignmentInfo>;
}

/// Step that inserts a new assignment row.
pub struct Create<'a> {
    pub form: &'a AssignmentForm,
}

impl<'a> Step for Create<'a> {
    type Output = AssignmentInfo;
}

/// Step that updates assignment roles.
pub struct PutRoles<'a> {
    pub update: &'a AssignmentRoleUpdate,
}

impl<'a> Step for PutRoles<'a> {
    type Output = AssignmentInfo;
}

/// Factory for constructing assignment repository [`Step`] values.
pub struct AssignmentStep;

impl AssignmentStep {
    /// Constructs a step to find one assignment by chapter and user.
    pub fn get_info_by_chapter_user_id<'a>(
        chapter_id: &'a str,
        user_id: &'a str,
    ) -> GetInfoByChapterUserId<'a> {
        GetInfoByChapterUserId {
            chapter_id,
            user_id,
        }
    }

    /// Constructs a step to insert a new assignment.
    pub fn create<'a>(form: &'a AssignmentForm) -> Create<'a> {
        Create { form }
    }

    /// Constructs a step to update assignment roles.
    pub fn put_roles<'a>(update: &'a AssignmentRoleUpdate) -> PutRoles<'a> {
        PutRoles { update }
    }
}
