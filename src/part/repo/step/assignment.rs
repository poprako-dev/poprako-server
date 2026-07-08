//! Step types for assignment repository opers.

use poprako_transactional::step::Step;

use crate::model::assignment::{
    AssignmentForm, AssignmentInfo, AssignmentListSpec, AssignmentRoleUpdate,
};
use crate::value::assignment::AssignmentInclOpt;
use crate::value::role::RoleField;

/// Step that finds one assignment by chapter ID and user ID.
pub struct GetInfoByChapterIdAndUserId<'a> {
    pub chapter_id: &'a str,
    pub user_id: &'a str,
}

impl<'a> Step for GetInfoByChapterIdAndUserId<'a> {
    type Output = Option<AssignmentInfo>;
}

/// Step that lists assignments by query specification.
pub struct ListInfos<'a> {
    pub spec: &'a AssignmentListSpec,
}

impl<'a> Step for ListInfos<'a> {
    type Output = Vec<AssignmentInfo>;
}

/// Step that lists all assignments by chapter (no pagination).
pub struct ListAllInfosByChapter<'a> {
    pub chapter_id: &'a str,
    pub role: Option<RoleField>,
    pub incl_opt: &'a [AssignmentInclOpt],
}

impl<'a> Step for ListAllInfosByChapter<'a> {
    type Output = Vec<AssignmentInfo>;
}

/// Step that fetches one assignment by its identifier.
pub struct GetInfoById<'a> {
    pub id: &'a str,
    pub incl_opt: &'a [AssignmentInclOpt],
}

impl<'a> Step for GetInfoById<'a> {
    type Output = AssignmentInfo;
}

/// Step that locks all assignment rows under one chapter.
pub struct ListInfosByChapterIdExcluded<'a> {
    pub chapter_id: &'a str,
}

impl<'a> Step for ListInfosByChapterIdExcluded<'a> {
    type Output = Vec<AssignmentInfo>;
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

/// Step that deletes one assignment by its identifier.
pub struct Delete<'a> {
    pub id: &'a str,
}

impl<'a> Step for Delete<'a> {
    type Output = ();
}

/// Step that deletes all assignments under one chapter.
pub struct DeleteByChapterId<'a> {
    pub chapter_id: &'a str,
}

impl<'a> Step for DeleteByChapterId<'a> {
    type Output = ();
}

/// Factory for constructing assignment repository [`Step`] values.
pub struct AssignmentStep;

impl AssignmentStep {
    /// Constructs a step to find one assignment by chapter ID and user ID.
    pub fn get_info_by_chapter_id_and_user_id<'a>(
        chapter_id: &'a str,
        user_id: &'a str,
    ) -> GetInfoByChapterIdAndUserId<'a> {
        GetInfoByChapterIdAndUserId {
            chapter_id,
            user_id,
        }
    }

    /// Constructs a step to list assignments.
    pub fn list_infos<'a>(spec: &'a AssignmentListSpec) -> ListInfos<'a> {
        ListInfos { spec }
    }

    /// Constructs a step to list all assignments by chapter (no pagination).
    pub fn list_all_infos_by_chapter<'a>(
        chapter_id: &'a str,
        role: Option<RoleField>,
        incl_opt: &'a [AssignmentInclOpt],
    ) -> ListAllInfosByChapter<'a> {
        ListAllInfosByChapter {
            chapter_id,
            role,
            incl_opt,
        }
    }

    /// Constructs a step to fetch one assignment by ID.
    pub fn get_info_by_id<'a>(id: &'a str, incl_opt: &'a [AssignmentInclOpt]) -> GetInfoById<'a> {
        GetInfoById { id, incl_opt }
    }

    /// Constructs a step to lock all assignments under one chapter.
    pub fn list_infos_by_chapter_id_excluded<'a>(
        chapter_id: &'a str,
    ) -> ListInfosByChapterIdExcluded<'a> {
        ListInfosByChapterIdExcluded { chapter_id }
    }

    /// Constructs a step to insert a new assignment.
    pub fn create<'a>(form: &'a AssignmentForm) -> Create<'a> {
        Create { form }
    }

    /// Constructs a step to update assignment roles.
    pub fn put_roles<'a>(update: &'a AssignmentRoleUpdate) -> PutRoles<'a> {
        PutRoles { update }
    }

    /// Constructs a step to delete one assignment.
    pub fn delete<'a>(id: &'a str) -> Delete<'a> {
        Delete { id }
    }

    /// Constructs a step to delete all assignments under one chapter.
    pub fn delete_by_chapter_id<'a>(chapter_id: &'a str) -> DeleteByChapterId<'a> {
        DeleteByChapterId { chapter_id }
    }
}
