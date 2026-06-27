//! Complex-domain operations for chapter assignments.

use time::OffsetDateTime;

use crate::model::assignment::{AssignmentInfo, AssignmentRoleUpdate};
use crate::model::role::{RoleBit, RoleMask};
use crate::util::next_snowflake_id;

/// Domain operations for chapter assignments.
pub struct AssignmentComplex;

impl AssignmentComplex {
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

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
                .has_any_role(&[RoleBit::RAW_PROVIDER])
                .then_some(now),
            role_mask
                .has_any_role(&[RoleBit::TRANSLATOR])
                .then_some(now),
            role_mask
                .has_any_role(&[RoleBit::PROOFREADER])
                .then_some(now),
            role_mask
                .has_any_role(&[RoleBit::TYPESETTER])
                .then_some(now),
            role_mask.has_any_role(&[RoleBit::REDRAWER]).then_some(now),
            role_mask.has_any_role(&[RoleBit::REVIEWER]).then_some(now),
            role_mask.has_any_role(&[RoleBit::PUBLISHER]).then_some(now),
        )
    }

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
