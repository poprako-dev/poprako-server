//! Step types for member repository operations.

use poprako_transactional::step::Step;

use crate::model::member::{MemberForm, MemberInfo};

/// Step that inserts a new membership row.
pub struct Create<'a> {
    pub form: &'a MemberForm,
}

impl<'a> Step for Create<'a> {
    type Output = MemberInfo;
}

/// Step that updates the cached nickname across a user's memberships.
pub struct UpdateUserNickname<'a> {
    pub user_id: &'a str,
    pub user_nickname: &'a str,
}

impl<'a> Step for UpdateUserNickname<'a> {
    type Output = ();
}

/// Step that updates the last-active timestamp on a user's memberships.
pub struct TouchLastActive<'a> {
    pub user_id: &'a str,
}

impl<'a> Step for TouchLastActive<'a> {
    type Output = ();
}

/// Step that lists all memberships for a user with a pessimistic lock.
pub struct ListByUserIdExcluded<'a> {
    pub user_id: &'a str,
}

impl<'a> Step for ListByUserIdExcluded<'a> {
    type Output = Vec<MemberInfo>;
}

/// Step that finds one membership by user and team identifiers.
pub struct FindByUserTeamId<'a> {
    pub user_id: &'a str,
    pub team_id: &'a str,
}

impl<'a> Step for FindByUserTeamId<'a> {
    type Output = Option<MemberInfo>;
}

/// Step that deletes a membership by its identifier.
pub struct Delete<'a> {
    pub id: &'a str,
}

impl<'a> Step for Delete<'a> {
    type Output = ();
}

/// Factory for constructing member repository [`Step`] values.
pub struct MemberStep;

impl MemberStep {
    /// Constructs a step to insert a new membership.
    pub fn create<'a>(form: &'a MemberForm) -> Create<'a> {
        Create { form }
    }

    /// Constructs a step to update the cached nickname for a user's memberships.
    pub fn update_user_nickname<'a>(
        user_id: &'a str,
        user_nickname: &'a str,
    ) -> UpdateUserNickname<'a> {
        UpdateUserNickname {
            user_id,
            user_nickname,
        }
    }

    /// Constructs a step to update last-active timestamps on memberships.
    pub fn touch_last_active<'a>(user_id: &'a str) -> TouchLastActive<'a> {
        TouchLastActive { user_id }
    }

    /// Constructs a step to list a user's memberships with a pessimistic lock.
    pub fn list_by_user_id_excluded<'a>(user_id: &'a str) -> ListByUserIdExcluded<'a> {
        ListByUserIdExcluded { user_id }
    }

    /// Constructs a step to find one membership by user and team.
    pub fn find_by_user_team_id<'a>(
        user_id: &'a str,
        team_id: &'a str,
    ) -> FindByUserTeamId<'a> {
        FindByUserTeamId { user_id, team_id }
    }

    /// Constructs a step to delete a membership.
    pub fn delete<'a>(id: &'a str) -> Delete<'a> {
        Delete { id }
    }
}
