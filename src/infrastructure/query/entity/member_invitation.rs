use diesel::prelude::*;
use time::OffsetDateTime;

use crate::domain::model::aggregate::member_invitation::MemberInvitation;
use crate::domain::model::value::role::RoleMask;
use crate::infrastructure::query::schema;

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
pub struct MemberInvitationEntry {
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

// ── Conversions ────────────────────────────────────────────────────────────

impl From<MemberInvitationRow> for MemberInvitation {
    fn from(v: MemberInvitationRow) -> Self {
        Self {
            id: v.f_id,
            invitor_id: v.f_inviter_id,
            invitor: None,
            team_id: v.f_team_id,
            invitee_qid: v.f_invitee_qid,
            code: v.f_invitation_code,
            pending: v.f_pending,
            roles: RoleMask::from(v.f_role_mask as u32),
            created_at: v.f_created_at,
        }
    }
}
