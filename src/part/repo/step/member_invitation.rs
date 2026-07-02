//! Step types for member invitation repository opers.

use poprako_transactional::step::Step;

use crate::model::member_invitation::{
    MemberInvitationForm, MemberInvitationInfo, MemberInvitationListSpec, MemberInvitationUpdate,
};
use crate::value::member_invitation::MemberInvitationInclOpt;

/// Step that inserts a member invitation row.
pub struct Create<'a> {
    pub form: &'a MemberInvitationForm,
}

impl<'a> Step for Create<'a> {
    type Output = MemberInvitationInfo;
}

/// Step that lists member invitations for a team with include options and pagination.
pub struct ListInfos<'a> {
    pub spec: &'a MemberInvitationListSpec,
}

impl<'a> Step for ListInfos<'a> {
    type Output = Vec<MemberInvitationInfo>;
}

/// Step that fetches an invitation by id.
pub struct GetInfoById<'a> {
    pub id: &'a str,
    pub incl_opt: &'a [MemberInvitationInclOpt],
}

impl<'a> Step for GetInfoById<'a> {
    type Output = MemberInvitationInfo;
}

/// Step that fetches a pending invitation by its code with a pessimistic lock.
pub struct GetInfoByCodeExcluded<'a> {
    pub code: &'a str,
}

impl<'a> Step for GetInfoByCodeExcluded<'a> {
    type Output = MemberInvitationInfo;
}

/// Step that marks a pending invitation as consumed.
pub struct MarkPendingAsUsed<'a> {
    pub id: &'a str,
}

impl<'a> Step for MarkPendingAsUsed<'a> {
    type Output = ();
}

/// Step that updates an invitation's roles.
pub struct UpdateInfo<'a> {
    pub update: &'a MemberInvitationUpdate,
}

impl<'a> Step for UpdateInfo<'a> {
    type Output = ();
}

/// Step that deletes an invitation row.
pub struct Delete<'a> {
    pub id: &'a str,
}

impl<'a> Step for Delete<'a> {
    type Output = ();
}

/// Factory for constructing member invitation repository [`Step`] values.
pub struct MemberInvitationStep;

impl MemberInvitationStep {
    /// Constructs a step to insert a member invitation.
    pub fn create<'a>(form: &'a MemberInvitationForm) -> Create<'a> {
        Create { form }
    }

    /// Constructs a step to list member invitations for a team.
    pub fn list_infos<'a>(spec: &'a MemberInvitationListSpec) -> ListInfos<'a> {
        ListInfos { spec }
    }

    /// Constructs a step to fetch an invitation by id.
    pub fn get_info_by_id<'a>(
        id: &'a str,
        incl_opt: &'a [MemberInvitationInclOpt],
    ) -> GetInfoById<'a> {
        GetInfoById { id, incl_opt }
    }

    /// Constructs a step to fetch a pending invitation by code with a lock.
    pub fn get_info_by_code_excluded<'a>(code: &'a str) -> GetInfoByCodeExcluded<'a> {
        GetInfoByCodeExcluded { code }
    }

    /// Constructs a step to mark a pending invitation as used.
    pub fn mark_pending_as_used<'a>(id: &'a str) -> MarkPendingAsUsed<'a> {
        MarkPendingAsUsed { id }
    }

    /// Constructs a step to update invitation info.
    pub fn update_info<'a>(update: &'a MemberInvitationUpdate) -> UpdateInfo<'a> {
        UpdateInfo { update }
    }

    /// Constructs a step to delete an invitation.
    pub fn delete<'a>(id: &'a str) -> Delete<'a> {
        Delete { id }
    }
}
