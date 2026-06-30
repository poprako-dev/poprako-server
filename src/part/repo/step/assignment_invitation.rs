//! Step types for assignment invitation repository opers.

use poprako_transactional::step::Step;

use crate::model::assignment_invitation::{AssignmentInvitationForm, AssignmentInvitationInfo};

/// Step that inserts an assignment invitation row.
pub struct Create<'a> {
    pub form: &'a AssignmentInvitationForm,
}

impl<'a> Step for Create<'a> {
    type Output = AssignmentInvitationInfo;
}

/// Step that lists assignment invitations under one chapter.
pub struct ListInfos<'a> {
    pub chapter_id: &'a str,
    pub pending: Option<bool>,
    pub offset: u64,
    pub limit: u64,
}

impl<'a> Step for ListInfos<'a> {
    type Output = Vec<AssignmentInvitationInfo>;
}

/// Step that fetches one assignment invitation by id.
pub struct GetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoById<'a> {
    type Output = AssignmentInvitationInfo;
}

/// Step that fetches a pending invitation by code with a pessimistic lock.
pub struct GetInfoByCodeExcluded<'a> {
    pub code: &'a str,
}

impl<'a> Step for GetInfoByCodeExcluded<'a> {
    type Output = AssignmentInvitationInfo;
}

/// Step that marks a pending invitation as consumed.
pub struct MarkPendingAsUsed<'a> {
    pub id: &'a str,
}

impl<'a> Step for MarkPendingAsUsed<'a> {
    type Output = ();
}

/// Step that deletes an invitation row.
pub struct Delete<'a> {
    pub id: &'a str,
}

impl<'a> Step for Delete<'a> {
    type Output = ();
}

/// Factory for constructing assignment invitation repository [`Step`] values.
pub struct AssignmentInvitationStep;

impl AssignmentInvitationStep {
    /// Constructs a step to insert an assignment invitation.
    pub fn create<'a>(form: &'a AssignmentInvitationForm) -> Create<'a> {
        Create { form }
    }

    /// Constructs a step to list assignment invitations under one chapter.
    pub fn list_infos<'a>(
        chapter_id: &'a str,
        pending: Option<bool>,
        offset: u64,
        limit: u64,
    ) -> ListInfos<'a> {
        ListInfos {
            chapter_id,
            pending,
            offset,
            limit,
        }
    }

    /// Constructs a step to fetch one invitation by id.
    pub fn get_info_by_id<'a>(id: &'a str) -> GetInfoById<'a> {
        GetInfoById { id }
    }

    /// Constructs a step to fetch a pending invitation by code with a lock.
    pub fn get_info_by_code_excluded<'a>(invitation_code: &'a str) -> GetInfoByCodeExcluded<'a> {
        GetInfoByCodeExcluded { code: invitation_code }
    }

    /// Constructs a step to mark a pending invitation as used.
    pub fn mark_pending_as_used<'a>(id: &'a str) -> MarkPendingAsUsed<'a> {
        MarkPendingAsUsed { id }
    }

    /// Constructs a step to delete an invitation.
    pub fn delete<'a>(id: &'a str) -> Delete<'a> {
        Delete { id }
    }
}
