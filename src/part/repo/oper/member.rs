use poprako_orchestra::Oper;

use crate::model::member::{
    MemberEntry, MemberInfo, MemberListSpec, MemberRoleUpdate,
};
use crate::value::member::MemberInclOpt;

/// Creates a new team member record.
pub struct CreateMember<'a> {
    /// The member entry to insert.
    pub entry: &'a MemberEntry,
}

impl Oper for CreateMember<'_> {
    // The created member info.
    type Output = MemberInfo;
}

/// Updates a member's fields (nickname or role).
pub enum UpdateMember<'a> {
    /// Updates the member's nickname.
    UserNickname {
        //
        /// The member's user ID.
        user_id: &'a str,
        /// The new nickname.
        user_nickname: &'a str,
    },

    /// Updates the member's role.
    Role {
        /// The role update payload.
        update: &'a MemberRoleUpdate,
    },
}

impl Oper for UpdateMember<'_> {
    // Unit on success.
    type Output = ();
}

/// Lists member infos by query spec or by user ID.
pub enum ListMemberInfos<'a> {
    /// Lists members matching the given spec.
    Spec {
        /// The filter and pagination specification.
        spec: &'a MemberListSpec,
    },

    /// Lists all memberships for a user.
    User {
        /// The user ID.
        user_id: &'a str,
    },
}

impl Oper for ListMemberInfos<'_> {
    // List of matching member infos.
    type Output = Vec<MemberInfo>;
}

/// Finds a single member by user and team.
pub enum FindMemberInfo<'a> {
    /// Finds by user ID and team ID.
    UserTeam {
        //
        /// The user ID.
        user_id: &'a str,
        /// The team ID.
        team_id: &'a str,
    },
}

impl Oper for FindMemberInfo<'_> {
    // The matching member info, if found.
    type Output = Option<MemberInfo>;
}

/// Retrieves a single member's info by ID with optional includes.
pub enum GetMemberInfo<'a, 'b> {
    /// Retrieves by member ID.
    Id {
        //
        /// The member ID.
        id: &'a str,
        /// Which relations to include in the response.
        incls: &'b [MemberInclOpt],
    },
}

impl Oper for GetMemberInfo<'_, '_> {
    // The retrieved member info.
    type Output = MemberInfo;
}

/// Lists member infos for a user or team with excluded fields omitted.
pub enum ListMemberInfosExcluded<'a> {
    /// Lists memberships for a user with excluded fields omitted.
    User {
        /// The user ID.
        user_id: &'a str,
    },

    /// Lists members for a team with excluded fields omitted.
    Team {
        /// The team ID.
        team_id: &'a str,
    },
}

impl Oper for ListMemberInfosExcluded<'_> {
    // List of matching member infos with excluded fields omitted.
    type Output = Vec<MemberInfo>;
}

/// Deletes a member record by ID.
pub struct DeleteMember<'a> {
    /// The member ID to delete.
    pub id: &'a str,
}

impl Oper for DeleteMember<'_> {
    // Unit on success.
    type Output = ();
}
