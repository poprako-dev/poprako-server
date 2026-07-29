//! Diesel entity types for the `t_assignment_invitation` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::assignment_invitation::{
    AssignmentInvitationEntry, AssignmentInvitationInfo,
};
use crate::part_impl::repo::rdb_impl::schema::t_assignment_invitation;
use crate::result::BaseError;
use crate::value::role::RoleMask;

/// Raw database row for the `t_assignment_invitation` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_assignment_invitation)]
pub struct AssignmentInvitationRow {
    //
    pub f_id: String,

    pub f_chapter_id: String,

    pub f_inviter_id: String,
    pub f_invitee_qid: String,

    pub f_code: String,

    pub f_pending: bool,
    pub f_role_mask: i64,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl TryFrom<AssignmentInvitationRow> for AssignmentInvitationInfo {
    type Error = BaseError;

    fn try_from(row: AssignmentInvitationRow) -> Result<Self, Self::Error> {
        //
        let roles = RoleMask::try_from(row.f_role_mask as u32)?;

        Ok(Self {
            id: row.f_id,
            chapter_id: row.f_chapter_id,
            inviter_id: row.f_inviter_id,
            invitee_qid: row.f_invitee_qid,
            code: row.f_code,
            pending: row.f_pending,
            roles,
            created_at: row.f_created_at,
            updated_at: row.f_updated_at,
        })
    }
}

/// Insertable struct for creating a new record in the `t_assignment_invitation` table.
#[derive(Insertable)]
#[diesel(table_name = t_assignment_invitation)]
pub struct AssignmentInvitationRowEntry<'a> {
    //
    pub f_id: &'a str,

    pub f_chapter_id: &'a str,

    pub f_inviter_id: &'a str,
    pub f_invitee_qid: &'a str,

    pub f_code: &'a str,

    pub f_pending: bool,
    pub f_role_mask: i64,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl<'a> From<&'a AssignmentInvitationEntry>
    for AssignmentInvitationRowEntry<'a>
{
    fn from(entry: &'a AssignmentInvitationEntry) -> Self {
        //
        let now = OffsetDateTime::now_utc();

        Self {
            f_id: &entry.id,
            f_chapter_id: &entry.chapter_id,
            f_inviter_id: &entry.inviter_id,
            f_invitee_qid: &entry.invitee_qid,
            f_code: &entry.code,
            f_pending: true,
            f_role_mask: i64::from(u32::from(entry.roles)),
            f_created_at: now,
            f_updated_at: now,
        }
    }
}

/// Aspect struct for updating specific fields of an assignment-invitation record
/// identified by id.
#[derive(AsChangeset)]
#[diesel(table_name = t_assignment_invitation)]
pub struct AssignmentInvitationAspect {
    //
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
        //
        self.f_pending = Some(val);

        self
    }
}
