use diesel::prelude::*;
use time::OffsetDateTime;

use crate::domain::model::aggr::member_invitation::MemberInvitationAggr;
use crate::domain::model::value::role::RoleMask;
use crate::infra::query::schema;

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

// ── Update Aspect ──────────────────────────────────────────────────────────

#[derive(AsChangeset)]
#[diesel(table_name = schema::t_member_invitation)]
pub struct MemberInvitationAspect {
    pub f_pending: bool,
    pub f_updated_at: OffsetDateTime,
}

// ── Conversions ────────────────────────────────────────────────────────────

impl From<MemberInvitationRow> for MemberInvitationAggr {
    fn from(v: MemberInvitationRow) -> Self {
        MemberInvitationAggr {
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
