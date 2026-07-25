use poprako_orchestra::Oper;

use crate::model::assignment::{
    AssignmentEntry, AssignmentInfo, AssignmentInfoListSpec,
    AssignmentRoleUpdate,
};
use crate::value::assignment::AssignmentInclOpt;
use crate::value::role::RoleField;

/// Finds an optional assignment.
pub enum FindAssignmentInfo<'a, 'b> {
    ChapterUser {
        //
        chapter_id: &'a str,
        user_id: &'a str,
    },

    UserComic {
        //
        user_id: &'a str,
        comic_id: &'a str,
        incls: &'b [AssignmentInclOpt],
    },
}

impl Oper for FindAssignmentInfo<'_, '_> {
    type Output = Option<AssignmentInfo>;
}

/// Gets an assignment that must exist.
pub struct GetAssignmentInfo<'a, 'b> {
    //
    pub id: &'a str,
    pub incls: &'b [AssignmentInclOpt],
}

impl Oper for GetAssignmentInfo<'_, '_> {
    type Output = AssignmentInfo;
}

/// Lists assignments selected by a query specification or chapter set.
pub enum ListAssignmentInfos<'a, 'b> {
    Spec {
        spec: &'a AssignmentInfoListSpec,
    },

    Chapter {
        //
        chapter_id: &'a str,
        role: Option<RoleField>,
        incls: &'b [AssignmentInclOpt],
    },

    Chapters {
        //
        chapter_ids: &'a [String],
        incls: &'b [AssignmentInclOpt],
    },
}

impl Oper for ListAssignmentInfos<'_, '_> {
    type Output = Vec<AssignmentInfo>;
}

/// Lists and exclusively locks all assignment rows under a chapter.
pub enum ListAssignmentInfosExcluded<'a> {
    Chapter { chapter_id: &'a str },
}

impl Oper for ListAssignmentInfosExcluded<'_> {
    type Output = Vec<AssignmentInfo>;
}

/// Creates an assignment.
pub struct CreateAssignment<'a> {
    pub entry: &'a AssignmentEntry,
}

impl Oper for CreateAssignment<'_> {
    type Output = AssignmentInfo;
}

/// Replaces the roles assigned to an assignment.
pub struct UpdateAssignmentRoles<'a> {
    pub update: &'a AssignmentRoleUpdate,
}

impl Oper for UpdateAssignmentRoles<'_> {
    type Output = AssignmentInfo;
}

/// Deletes assignments selected by identifier or chapter.
pub enum DeleteAssignments<'a> {
    Id { id: &'a str },

    Chapter { chapter_id: &'a str },
}

impl Oper for DeleteAssignments<'_> {
    type Output = ();
}
