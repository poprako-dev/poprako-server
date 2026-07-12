//! Step types for member repository opers.

use poprako_transactional::step::Step;

use crate::model::member_model;
use crate::value::member::MemberInclOpt;

/// Step that inserts a new membership row.
pub struct Create<'a> {
    pub form: &'a member_model::Form,
}

impl<'a> Step for Create<'a> {
    type Output = member_model::Info;
}

/// Step that updates the cached nickname across a user's memberships.
pub struct UpdateUserNickname<'a> {
    pub user_id: &'a str,
    pub user_nickname: &'a str,
}

impl<'a> Step for UpdateUserNickname<'a> {
    type Output = ();
}

/// Step that lists all memberships for a user with a pessimistic lock.
pub struct ListInfosByUserIdExcluded<'a> {
    pub user_id: &'a str,
}

impl<'a> Step for ListInfosByUserIdExcluded<'a> {
    type Output = Vec<member_model::Info>;
}

/// Step that lists memberships under one team.
pub struct ListInfos<'a> {
    pub spec: &'a member_model::ListSpec,
}

impl<'a> Step for ListInfos<'a> {
    type Output = Vec<member_model::Info>;
}

/// Step that finds one membership by user ID and team ID.
pub struct FindInfoByUserIdAndTeamId<'a> {
    pub user_id: &'a str,
    pub team_id: &'a str,
}

impl<'a> Step for FindInfoByUserIdAndTeamId<'a> {
    type Output = Option<member_model::Info>;
}

/// Step that fetches one membership by ID.
pub struct GetInfoById<'a> {
    pub id: &'a str,
    pub incl_opt: &'a [MemberInclOpt],
}

impl<'a> Step for GetInfoById<'a> {
    type Output = member_model::Info;
}

/// Step that updates one membership's roles.
pub struct UpdateRole<'a> {
    pub member_role_update: &'a member_model::RoleUpdate,
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
    pub fn create<'a>(form: &'a member_model::Form) -> Create<'a> {
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

    /// Constructs a step to list a user's memberships with a pessimistic lock.
    pub fn list_infos_by_user_id_excluded<'a>(
        user_id: &'a str,
    ) -> ListInfosByUserIdExcluded<'a> {
        ListInfosByUserIdExcluded { user_id }
    }

    /// Constructs a step to list team memberships.
    pub fn list_infos<'a>(spec: &'a member_model::ListSpec) -> ListInfos<'a> {
        ListInfos { spec }
    }

    /// Constructs a step to find one membership by user ID and team ID.
    pub fn find_info_by_user_id_and_team_id<'a>(
        user_id: &'a str,
        team_id: &'a str,
    ) -> FindInfoByUserIdAndTeamId<'a> {
        FindInfoByUserIdAndTeamId { user_id, team_id }
    }

    /// Constructs a step to fetch one membership by ID.
    pub fn get_info_by_id<'a>(
        id: &'a str,
        incl_opt: &'a [MemberInclOpt],
    ) -> GetInfoById<'a> {
        GetInfoById { id, incl_opt }
    }

    /// Constructs a step to update a member's roles.
    pub fn update_role<'a>(
        member_role_update: &'a member_model::RoleUpdate,
    ) -> UpdateRole<'a> {
        UpdateRole { member_role_update }
    }

    /// Constructs a step to delete a membership.
    pub fn delete<'a>(id: &'a str) -> Delete<'a> {
        Delete { id }
    }
}
