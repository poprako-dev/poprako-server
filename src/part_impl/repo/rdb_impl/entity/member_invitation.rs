//! Diesel entity types for the `t_member_invitation` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::member_invitation::{MemberInvitationEntry, MemberInvitationInfo};
use crate::part_impl::repo::rdb_impl::schema::t_member_invitation;
use crate::result::BaseError;
use crate::value::role::RoleMask;

// ── Queryable / Selectable ─────────────────────────────────────────────────

/// Raw database row for the `t_member_invitation` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_member_invitation)]
pub struct MemberInvitationRow {
    pub f_id: String,
    pub f_inviter_id: String,
    pub f_team_id: String,
    pub f_invitee_qid: String,

    pub f_code: String,

    pub f_pending: bool,
    pub f_role_mask: i64,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Insertable ─────────────────────────────────────────────────────────────

/// Insertable struct for creating a new record in the `t_member_invitation` table.
#[derive(Insertable)]
#[diesel(table_name = t_member_invitation)]
pub struct MemberInvitationRowEntry<'a> {
    pub f_id: &'a str,
    pub f_inviter_id: &'a str,
    pub f_team_id: &'a str,
    pub f_invitee_qid: &'a str,

    pub f_code: &'a str,

    pub f_pending: bool,
    pub f_role_mask: i64,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Changeset (AsChangeset) ────────────────────────────────────────────────

/// Aspect struct for updating specific fields of a member-invitation record
/// identified by id.
#[derive(AsChangeset)]
#[diesel(table_name = t_member_invitation)]
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
        //
        self.f_pending = Some(val);

        self
    }

    pub fn role_mask(mut self, val: i64) -> Self {
        //
        self.f_role_mask = Some(val);

        self
    }
}

// ── Conversions ────────────────────────────────────────────────────────────

impl TryFrom<MemberInvitationRow> for MemberInvitationInfo {
    type Error = BaseError;

    fn try_from(v: MemberInvitationRow) -> Result<Self, Self::Error> {
        //
        let roles = RoleMask::try_from(v.f_role_mask as u32)?;

        Ok(MemberInvitationInfo {
            id: v.f_id,
            team_id: v.f_team_id,
            invitor: None,
            invitor_id: v.f_inviter_id,
            invitee_qid: v.f_invitee_qid,
            code: v.f_code,
            pending: v.f_pending,
            roles,
        })
    }
}

impl<'a> From<&'a MemberInvitationEntry> for MemberInvitationRowEntry<'a> {
    fn from(entry: &'a MemberInvitationEntry) -> Self {
        Self {
            f_id: &entry.id,
            f_inviter_id: &entry.invitor_id,
            f_team_id: &entry.team_id,
            f_invitee_qid: &entry.invitee_qid,
            f_code: &entry.code,
            f_pending: true,
            f_role_mask: i64::from(u32::from(entry.roles)),
            f_created_at: OffsetDateTime::now_utc(),
            f_updated_at: OffsetDateTime::now_utc(),
        }
    }
}
