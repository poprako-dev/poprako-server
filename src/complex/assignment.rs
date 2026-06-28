//! Complex-domain operations for chapter assignments.

use time::OffsetDateTime;

use crate::model::assignment::{AssignmentInfo, AssignmentRoleUpdate};
use crate::model::role::{RoleField, RoleMask};
use crate::util::next_snowflake_id;

/// Domain operations for chapter assignments: ID generation, role-timestamp
/// derivation, and role-merge logic.
pub struct AssignmentComplex;

impl AssignmentComplex {
    /// Generate a unique assignment identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Derive per-role `assigned_at` timestamps from a [`RoleMask`].
    ///
    /// Each role bit present in the mask yields `Some(now)` for the
    /// corresponding timestamp field; absent bits yield `None`.
    /// Order follows: `RAW_PROVIDER`, `TRANSLATOR`, `PROOFREADER`,
    /// `TYPESETTER`, `REDRAWER`, `REVIEWER`, `PUBLISHER`.
    pub fn timed_roles_from_mask(
        role_mask: RoleMask,
        now: OffsetDateTime,
    ) -> (
        Option<OffsetDateTime>,
        Option<OffsetDateTime>,
        Option<OffsetDateTime>,
        Option<OffsetDateTime>,
        Option<OffsetDateTime>,
        Option<OffsetDateTime>,
        Option<OffsetDateTime>,
    ) {
        (
            role_mask
                .has_any_role(&[RoleField::RAW_PROVIDER])
                .then_some(now),
            role_mask
                .has_any_role(&[RoleField::TRANSLATOR])
                .then_some(now),
            role_mask
                .has_any_role(&[RoleField::PROOFREADER])
                .then_some(now),
            role_mask
                .has_any_role(&[RoleField::TYPESETTER])
                .then_some(now),
            role_mask
                .has_any_role(&[RoleField::REDRAWER])
                .then_some(now),
            role_mask
                .has_any_role(&[RoleField::REVIEWER])
                .then_some(now),
            role_mask
                .has_any_role(&[RoleField::PUBLISHER])
                .then_some(now),
        )
    }

    /// Merge new roles into an existing assignment, preserving existing
    /// `assigned_at` timestamps for roles already held and writing `now`
    /// for newly granted roles.
    pub fn merge_timed_roles(
        assignment_info: &AssignmentInfo,
        role_mask: RoleMask,
        now: OffsetDateTime,
    ) -> AssignmentRoleUpdate {
        let merged_role_mask = assignment_info.role_mask.union(role_mask);
        let (
            raw_provider_assigned_at,
            translator_assigned_at,
            proofreader_assigned_at,
            typesetter_assigned_at,
            redrawer_assigned_at,
            reviewer_assigned_at,
            publisher_assigned_at,
        ) = Self::timed_roles_from_mask(role_mask, now);

        AssignmentRoleUpdate {
            id: assignment_info.id.clone(),
            role_mask: merged_role_mask,
            raw_provider_assigned_at: assignment_info
                .raw_provider_assigned_at
                .or(raw_provider_assigned_at),
            translator_assigned_at: assignment_info
                .translator_assigned_at
                .or(translator_assigned_at),
            proofreader_assigned_at: assignment_info
                .proofreader_assigned_at
                .or(proofreader_assigned_at),
            typesetter_assigned_at: assignment_info
                .typesetter_assigned_at
                .or(typesetter_assigned_at),
            redrawer_assigned_at: assignment_info
                .redrawer_assigned_at
                .or(redrawer_assigned_at),
            reviewer_assigned_at: assignment_info
                .reviewer_assigned_at
                .or(reviewer_assigned_at),
            publisher_assigned_at: assignment_info
                .publisher_assigned_at
                .or(publisher_assigned_at),
        }
    }
}
