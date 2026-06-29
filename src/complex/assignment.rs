//! Complex-domain opers for chapter assignments.

use crate::model::assignment::{AssignmentInfo, AssignmentRoleUpdate};
use crate::model::role::RoleMask;
use crate::util::next_snowflake_id;

/// Domain opers for chapter assignments: ID generation and role-merge logic.
pub struct AssignmentComplex;

impl AssignmentComplex {
    /// Generate a unique assignment identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Merge new roles into an existing assignment, preserving existing roles
    /// and writing new ones.
    pub fn merge_roles(
        assignment_info: &AssignmentInfo,
        role_mask: RoleMask,
    ) -> AssignmentRoleUpdate {
        AssignmentRoleUpdate {
            id: assignment_info.id.clone(),
            roles: assignment_info.roles.union(role_mask),
        }
    }
}
