//! Step types for member repository opers.

use poprako_transactional::step::Step;

use crate::model::member::{MemberForm, MemberInfo, MemberListSpec, MemberRoleUpdate};

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
pub struct ListInfosByUserIdExcluded<'a> {
    pub user_id: &'a str,
}

impl<'a> Step for ListInfosByUserIdExcluded<'a> {
    type Output = Vec<MemberInfo>;
}

/// Step that lists memberships under one team.
pub struct ListInfos<'a> {
    pub spec: &'a MemberListSpec,
}

impl<'a> Step for ListInfos<'a> {
    type Output = Vec<MemberInfo>;
}

/// Step that finds one membership by user ID and team ID.
pub struct FindInfoByUserIdAndTeamId<'a> {
    pub user_id: &'a str,
    pub team_id: &'a str,
}

impl<'a> Step for FindInfoByUserIdAndTeamId<'a> {
    type Output = Option<MemberInfo>;
}

/// Step that fetches one membership by ID with a pessimistic lock.
pub struct GetInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoExcluded<'a> {
    type Output = MemberInfo;
}

/// Step that fetches one membership by ID.
pub struct GetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoById<'a> {
    type Output = MemberInfo;
}

/// Step that updates one membership's role mask.
pub struct UpdateRole<'a> {
    pub member_role_update: &'a MemberRoleUpdate,
}

impl<'a> Step for UpdateRole<'a> {
    type Output = ();
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
    pub fn list_infos_by_user_id_excluded<'a>(user_id: &'a str) -> ListInfosByUserIdExcluded<'a> {
        ListInfosByUserIdExcluded { user_id }
    }

    /// Constructs a step to list team memberships.
    pub fn list_infos<'a>(spec: &'a MemberListSpec) -> ListInfos<'a> {
        ListInfos { spec }
    }

    /// Constructs a step to find one membership by user ID and team ID.
    pub fn find_info_by_user_id_and_team_id<'a>(
        user_id: &'a str,
        team_id: &'a str,
    ) -> FindInfoByUserIdAndTeamId<'a> {
        FindInfoByUserIdAndTeamId { user_id, team_id }
    }

    /// Constructs a step to fetch one membership with a pessimistic lock.
    pub fn get_info_excluded<'a>(id: &'a str) -> GetInfoExcluded<'a> {
        GetInfoExcluded { id }
    }

    /// Constructs a step to fetch one membership by ID.
    pub fn get_info_by_id<'a>(id: &'a str) -> GetInfoById<'a> {
        GetInfoById { id }
    }

    /// Constructs a step to update a member role mask.
    pub fn update_role<'a>(member_role_update: &'a MemberRoleUpdate) -> UpdateRole<'a> {
        UpdateRole { member_role_update }
    }

    /// Constructs a step to delete a membership.
    pub fn delete<'a>(id: &'a str) -> Delete<'a> {
        Delete { id }
    }
}
