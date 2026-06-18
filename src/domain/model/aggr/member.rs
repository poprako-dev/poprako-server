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

    pub role_mask: RoleMask,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::value::role::RoleFlag;
    use time::OffsetDateTime;

    fn make_member_with_roles(roles: &[(RoleFlag, Option<OffsetDateTime>)]) -> MemberAggr {
        let now = OffsetDateTime::now_utc();
        let mut m = MemberAggr {
            id: "m-test".into(),
            user_id: "u-1".into(),
            user_nickname: "Test".into(),
            user: None,
            team_id: "t-1".into(),
            team: None,
            assigned_raw_provider_at: None,
            assigned_translator_at: None,
            assigned_proofreader_at: None,
            assigned_typesetter_at: None,
            assigned_redrawer_at: None,
            assigned_reviewer_at: None,
            assigned_publisher_at: None,
            assigned_admin_at: None,
            assigned_assistant_at: None,
            user_last_active_at: now,
            created_at: now,
            updated_at: now,
        };
        for (flag, ts) in roles {
            match flag {
                RoleFlag::RawProvider => m.assigned_raw_provider_at = *ts,
                RoleFlag::Translator => m.assigned_translator_at = *ts,
                RoleFlag::Proofreader => m.assigned_proofreader_at = *ts,
                RoleFlag::Typesetter => m.assigned_typesetter_at = *ts,
                RoleFlag::Redrawer => m.assigned_redrawer_at = *ts,
                RoleFlag::Reviewer => m.assigned_reviewer_at = *ts,
                RoleFlag::Publisher => m.assigned_publisher_at = *ts,
                RoleFlag::Admin => m.assigned_admin_at = *ts,
                RoleFlag::Assistant => m.assigned_assistant_at = *ts,
            }
        }
        m
    }

    #[test]
    fn role_mask_with_all_roles_includes_every_flag() {
        let now = OffsetDateTime::now_utc();
        let flags = [
            RoleFlag::RawProvider,
            RoleFlag::Translator,
            RoleFlag::Proofreader,
            RoleFlag::Typesetter,
            RoleFlag::Redrawer,
            RoleFlag::Reviewer,
            RoleFlag::Publisher,
            RoleFlag::Admin,
            RoleFlag::Assistant,
        ];
        let roles: Vec<_> = flags.iter().map(|f| (*f, Some(now))).collect();
        let m = make_member_with_roles(&roles);
        let mask: u32 = m.role_mask().into();
        for flag in &flags {
            assert!(mask & u32::from(*flag) != 0, "missing {flag:?}");
        }
    }

    #[test]
    fn has_any_role_checks_any_flag() {
        let now = OffsetDateTime::now_utc();
        let m = make_member_with_roles(&[(RoleFlag::Admin, Some(now))]);
        assert!(m.has_any_role(&[RoleFlag::Admin]));
        assert!(m.has_any_role(&[RoleFlag::Admin, RoleFlag::Translator]));
        assert!(!m.has_any_role(&[RoleFlag::Translator]));
    }

    #[test]
    fn has_every_role_checks_all_flags() {
        let now = OffsetDateTime::now_utc();
        let m = make_member_with_roles(&[
            (RoleFlag::Admin, Some(now)),
            (RoleFlag::Translator, Some(now)),
        ]);
        assert!(m.has_every_role(&[RoleFlag::Admin, RoleFlag::Translator]));
        assert!(!m.has_every_role(&[RoleFlag::Admin, RoleFlag::Proofreader]));
    }

    #[test]
    fn generate_id_produces_prefixed_uuid() {
        let id = MemberAggr::generate_id();
        assert!(id.starts_with("member-"));
    }
}

/// Input aggregate for updating the roles of an existing member (PUT semantics).
///
/// The caller provides the existing member `id` and the target [`RoleMask`].
pub struct MemberRoleUpdate {
    pub id: String,

    pub role_mask: RoleMask,
}
