use poprako_orchestra::Oper;

use crate::model::assignment::{
    AssignmentEntry, AssignmentInfo, AssignmentInfoListSpec,
    AssignmentRoleUpdate,
};
use crate::value::assignment::AssignmentInclOpt;
use crate::value::role::RoleField;

/// Finds an optional assignment.
pub enum FindAssignmentInfo<'a, 'b> {
    /// Finds by chapter and user.
    ChapterUser {
        //
        /// Chapter identifier.
        chapter_id: &'a str,
        /// User identifier.
        user_id: &'a str,
    },

    /// Finds by user and comic.
    UserComic {
        //
        /// User identifier.
        user_id: &'a str,
        /// Comic identifier.
        comic_id: &'a str,
        /// Assignment inclusion options.
        incls: &'b [AssignmentInclOpt],
    },
}

impl Oper for FindAssignmentInfo<'_, '_> {
    // Internal output type for this step.
    type Output = Option<AssignmentInfo>;
}

/// Gets an assignment that must exist.
pub struct GetAssignmentInfo<'a, 'b> {
    //
    /// Assignment identifier.
    pub id: &'a str,
    /// Assignment inclusion options.
    pub incls: &'b [AssignmentInclOpt],
}

impl Oper for GetAssignmentInfo<'_, '_> {
    // Internal output type for this step.
    type Output = AssignmentInfo;
}

/// Lists assignments selected by a query specification or chapter set.
pub enum ListAssignmentInfos<'a, 'b> {
    /// Lists by a query specification.
    Spec {
        /// Query specification for filtering assignments.
        spec: &'a AssignmentInfoListSpec,
    },

    /// Lists by a single chapter.
    Chapter {
        //
        /// Chapter identifier.
        chapter_id: &'a str,
        /// Optional role filter.
        role: Option<RoleField>,
        /// Assignment inclusion options.
        incls: &'b [AssignmentInclOpt],
    },

    /// Lists by a set of chapters.
    Chapters {
        //
        /// Chapter identifiers.
        chapter_ids: &'a [String],
        /// Assignment inclusion options.
        incls: &'b [AssignmentInclOpt],
    },
}

impl Oper for ListAssignmentInfos<'_, '_> {
    // Internal output type for this step.
    type Output = Vec<AssignmentInfo>;
}

/// Lists and exclusively locks all assignment rows under a chapter.
pub enum ListAssignmentInfosExcluded<'a> {
    /// Lists and locks by chapter.
    Chapter {
        /// Chapter identifier.
        chapter_id: &'a str,
    },
}

impl Oper for ListAssignmentInfosExcluded<'_> {
    // Internal output type for this step.
    type Output = Vec<AssignmentInfo>;
}

/// Creates an assignment.
pub struct CreateAssignment<'a> {
    /// The assignment entry data.
    pub entry: &'a AssignmentEntry,
}

impl Oper for CreateAssignment<'_> {
    // Internal output type for this step.
    type Output = AssignmentInfo;
}

/// Replaces the roles assigned to an assignment.
pub struct UpdateAssignmentRoles<'a> {
    /// The role update data.
    pub update: &'a AssignmentRoleUpdate,
}

impl Oper for UpdateAssignmentRoles<'_> {
    // Internal output type for this step.
    type Output = AssignmentInfo;
}

/// Deletes assignments selected by identifier or chapter.
pub enum DeleteAssignments<'a> {
    /// Deletes by assignment identifier.
    Id {
        /// Assignment identifier.
        id: &'a str,
    },

    /// Deletes by chapter.
    Chapter {
        /// Chapter identifier.
        chapter_id: &'a str,
    },
}

impl Oper for DeleteAssignments<'_> {
    // Internal output type for this step.
    type Output = ();
}
