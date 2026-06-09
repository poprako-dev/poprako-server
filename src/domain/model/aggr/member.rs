use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::model::aggr::team::TeamAggr;
use crate::domain::model::aggr::user::UserAggr;
use crate::domain::model::value::role::{RoleFlag, RoleMask};

#[cfg_attr(test, derive(Clone))]
pub struct MemberAggr {
    pub id: String,

    pub user_id: String,
    pub user_nickname: String,
    pub user: Option<UserAggr>,

    pub team_id: String,
    pub team: Option<TeamAggr>,

    pub assigned_raw_provider_at: Option<OffsetDateTime>,
    pub assigned_translator_at: Option<OffsetDateTime>,
    pub assigned_proofreader_at: Option<OffsetDateTime>,
    pub assigned_typesetter_at: Option<OffsetDateTime>,
    pub assigned_redrawer_at: Option<OffsetDateTime>,
    pub assigned_reviewer_at: Option<OffsetDateTime>,
    pub assigned_publisher_at: Option<OffsetDateTime>,
    pub assigned_admin_at: Option<OffsetDateTime>,
    pub assigned_assistant_at: Option<OffsetDateTime>,

    pub user_last_active_at: OffsetDateTime,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl MemberAggr {
    pub fn generate_id() -> String {
        format!("member-{}", Uuid::now_v7())
    }

    /// Builds a [`RoleMask`] from the non-None role timestamp fields.
    pub fn role_mask(&self) -> RoleMask {
        let mut bits: u32 = 0;

        if self.assigned_raw_provider_at.is_some() {
            bits |= u32::from(RoleFlag::RawProvider);
        }
        if self.assigned_translator_at.is_some() {
            bits |= u32::from(RoleFlag::Translator);
        }
        if self.assigned_proofreader_at.is_some() {
            bits |= u32::from(RoleFlag::Proofreader);
        }
        if self.assigned_typesetter_at.is_some() {
            bits |= u32::from(RoleFlag::Typesetter);
        }
        if self.assigned_redrawer_at.is_some() {
            bits |= u32::from(RoleFlag::Redrawer);
        }
        if self.assigned_reviewer_at.is_some() {
            bits |= u32::from(RoleFlag::Reviewer);
        }
        if self.assigned_publisher_at.is_some() {
            bits |= u32::from(RoleFlag::Publisher);
        }
        if self.assigned_admin_at.is_some() {
            bits |= u32::from(RoleFlag::Admin);
        }
        if self.assigned_assistant_at.is_some() {
            bits |= u32::from(RoleFlag::Assistant);
        }

        RoleMask::try_from(bits).expect("member role timestamps should produce a valid role mask")
    }

    /// Reports whether the member has at least one of the given roles.
    pub fn has_any_role(&self, flags: &[RoleFlag]) -> bool {
        self.role_mask().has_any_role(flags)
    }

    /// Reports whether the member has every one of the given roles.
    pub fn has_every_role(&self, flags: &[RoleFlag]) -> bool {
        self.role_mask().has_every_role(flags)
    }
}

pub struct MemberForm {
    pub id: String,

    pub user_id: String,
    pub user_nickname: String,

    pub team_id: String,

    pub roles: RoleMask,
}

/// Input aggregate for updating the roles of an existing member (PUT semantics).
///
/// The caller provides the existing member `id` and the target [`RoleMask`].
pub struct MemberRoleUpdate {
    pub id: String,

    pub roles: RoleMask,
}
