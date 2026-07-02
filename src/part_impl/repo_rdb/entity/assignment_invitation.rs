//! Diesel entity types for the `t_assignment_invitation` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::assignment_invitation::{AssignmentInvitationForm, AssignmentInvitationInfo};
use crate::part_impl::repo_rdb::schema::t_assignment_invitation;
use crate::result::RegularError;
use crate::value::role::RoleMask;

#[derive(Queryable, Selectable)]
#[diesel(table_name = t_assignment_invitation)]
pub struct AssignmentInvitationRow {
    pub f_id: String,

    pub f_chapter_id: String,

    pub f_inviter_id: String,
    pub f_invitee_qid: String,

    pub f_invitation_code: String,

    pub f_pending: bool,
    pub f_role_mask: i64,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = t_assignment_invitation)]
pub struct AssignmentInvitationEntry<'a> {
    pub f_id: &'a str,

    pub f_chapter_id: &'a str,

    pub f_inviter_id: &'a str,
    pub f_invitee_qid: &'a str,

    pub f_invitation_code: &'a str,

    pub f_pending: bool,
    pub f_role_mask: i64,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

#[derive(AsChangeset)]
#[diesel(table_name = t_assignment_invitation)]
pub struct AssignmentInvitationAspect {
    pub f_pending: Option<bool>,
    pub f_updated_at: OffsetDateTime,
}

impl AssignmentInvitationAspect {
    pub fn new(updated_at: OffsetDateTime) -> Self {
        Self {
            f_pending: None,
            f_updated_at: updated_at,
        }
    }

    pub fn pending(mut self, val: bool) -> Self {
        self.f_pending = Some(val);
        self
    }
}

impl TryFrom<AssignmentInvitationRow> for AssignmentInvitationInfo {
    type Error = RegularError;

    fn try_from(row: AssignmentInvitationRow) -> Result<Self, Self::Error> {
        let roles = RoleMask::try_from(row.f_role_mask as u32)?;

        Ok(Self {
            id: row.f_id,
            chapter_id: row.f_chapter_id,
            inviter_id: row.f_inviter_id,
            invitee_qid: row.f_invitee_qid,
            code: row.f_invitation_code,
            pending: row.f_pending,
            roles,
            created_at: row.f_created_at,
            updated_at: row.f_updated_at,
        })
    }
}

impl<'a> From<&'a AssignmentInvitationForm> for AssignmentInvitationEntry<'a> {
    fn from(form: &'a AssignmentInvitationForm) -> Self {
        let now = OffsetDateTime::now_utc();

        Self {
            f_id: &form.id,
            f_chapter_id: &form.chapter_id,
            f_inviter_id: &form.inviter_id,
            f_invitee_qid: &form.invitee_qid,
            f_invitation_code: &form.code,
            f_pending: true,
            f_role_mask: i64::from(u32::from(form.roles)),
            f_created_at: now,
            f_updated_at: now,
        }
    }
}
