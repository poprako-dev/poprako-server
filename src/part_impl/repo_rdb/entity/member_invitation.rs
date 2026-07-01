//! Diesel entity types for the `t_member_invitation` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::member_invitation::{MemberInvitationForm, MemberInvitationInfo};
use crate::part_impl::repo_rdb::schema;
use crate::value::role::RoleMask;

// ── Queryable / Selectable ─────────────────────────────────────────────────

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::t_member_invitation)]
pub struct MemberInvitationRow {
    pub f_id: String,
    pub f_inviter_id: String,
    pub f_team_id: String,
    pub f_invitee_qid: String,

    pub f_invitation_code: String,

    pub f_pending: bool,
    pub f_role_mask: i64,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Insertable ─────────────────────────────────────────────────────────────

#[derive(Insertable)]
#[diesel(table_name = schema::t_member_invitation)]
pub struct MemberInvitationEntry<'a> {
    pub f_id: &'a str,
    pub f_inviter_id: &'a str,
    pub f_team_id: &'a str,
    pub f_invitee_qid: &'a str,

    pub f_invitation_code: &'a str,

    pub f_pending: bool,
    pub f_role_mask: i64,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Changeset (AsChangeset) ────────────────────────────────────────────────

#[derive(AsChangeset)]
#[diesel(table_name = schema::t_member_invitation)]
pub struct MemberInvitationAspect {
    pub f_pending: Option<bool>,
    pub f_role_mask: Option<i64>,

    pub f_updated_at: OffsetDateTime,
}

impl MemberInvitationAspect {
    pub fn new(updated_at: OffsetDateTime) -> Self {
        Self {
            f_pending: None,
            f_role_mask: None,
            f_updated_at: updated_at,
        }
    }

    pub fn pending(mut self, val: bool) -> Self {
        self.f_pending = Some(val);
        self
    }

    pub fn role_mask(mut self, val: i64) -> Self {
        self.f_role_mask = Some(val);
        self
    }
}

// ── Conversions ────────────────────────────────────────────────────────────

impl TryFrom<MemberInvitationRow> for MemberInvitationInfo {
    type Error = crate::result::RegularError;

    fn try_from(v: MemberInvitationRow) -> Result<Self, Self::Error> {
        let roles = RoleMask::try_from(v.f_role_mask as u32)?;

        Ok(MemberInvitationInfo {
            id: v.f_id,
            team_id: v.f_team_id,
            invitor: None,
            invitor_id: v.f_inviter_id,
            invitee_qid: v.f_invitee_qid,
            code: v.f_invitation_code,
            pending: v.f_pending,
            roles,
        })
    }
}

impl<'a> From<&'a MemberInvitationForm> for MemberInvitationEntry<'a> {
    fn from(form: &'a MemberInvitationForm) -> Self {
        Self {
            f_id: &form.id,
            f_inviter_id: &form.invitor_id,
            f_team_id: &form.team_id,
            f_invitee_qid: &form.invitee_qid,
            f_invitation_code: &form.code,
            f_pending: true,
            f_role_mask: i64::from(u32::from(form.roles)),
            f_created_at: OffsetDateTime::now_utc(),
            f_updated_at: OffsetDateTime::now_utc(),
        }
    }
}
