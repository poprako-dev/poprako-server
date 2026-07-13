//! Diesel entity types for the `t_member` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::member::MemberInfo;
use crate::part_impl::repo::rdb_impl::schema::t_member;
use crate::value::role::{RoleField, RoleMask};

// ── Queryable / Selectable ─────────────────────────────────────────────────

/// Raw database row for the `t_member` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_member)]
pub struct MemberRow {
    pub f_id: String,
    pub f_user_id: String,
    pub f_user_nickname: String,
    pub f_team_id: String,

    pub f_assigned_raw_provider_at: Option<OffsetDateTime>,
    pub f_assigned_translator_at: Option<OffsetDateTime>,
    pub f_assigned_proofreader_at: Option<OffsetDateTime>,
    pub f_assigned_typesetter_at: Option<OffsetDateTime>,
    pub f_assigned_redrawer_at: Option<OffsetDateTime>,
    pub f_assigned_reviewer_at: Option<OffsetDateTime>,
    pub f_assigned_publisher_at: Option<OffsetDateTime>,
    pub f_assigned_admin_at: Option<OffsetDateTime>,
    pub f_assigned_bot_at: Option<OffsetDateTime>,

    pub f_user_last_active_at: OffsetDateTime,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Insertable ─────────────────────────────────────────────────────────────

/// Insertable struct for creating a new record in the `t_member` table.
#[derive(Insertable)]
#[diesel(table_name = t_member)]
pub struct MemberRowEntry<'a> {
    pub f_id: &'a str,
    pub f_user_id: &'a str,
    pub f_user_nickname: &'a str,
    pub f_team_id: &'a str,

    pub f_assigned_raw_provider_at: Option<OffsetDateTime>,
    pub f_assigned_translator_at: Option<OffsetDateTime>,
    pub f_assigned_proofreader_at: Option<OffsetDateTime>,
    pub f_assigned_typesetter_at: Option<OffsetDateTime>,
    pub f_assigned_redrawer_at: Option<OffsetDateTime>,
    pub f_assigned_reviewer_at: Option<OffsetDateTime>,
    pub f_assigned_publisher_at: Option<OffsetDateTime>,
    pub f_assigned_admin_at: Option<OffsetDateTime>,
    pub f_assigned_bot_at: Option<OffsetDateTime>,

    pub f_user_last_active_at: OffsetDateTime,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Changeset (AsChangeset) ────────────────────────────────────────────────

/// Aspect struct for updating specific fields of a member record identified by id.
#[derive(AsChangeset)]
#[diesel(table_name = t_member)]
pub struct MemberAspect<'a> {
    pub f_user_nickname: Option<&'a str>,

    pub f_user_last_active_at: Option<OffsetDateTime>,

    pub f_assigned_raw_provider_at: Option<Option<OffsetDateTime>>,
    pub f_assigned_translator_at: Option<Option<OffsetDateTime>>,
    pub f_assigned_proofreader_at: Option<Option<OffsetDateTime>>,
    pub f_assigned_typesetter_at: Option<Option<OffsetDateTime>>,
    pub f_assigned_redrawer_at: Option<Option<OffsetDateTime>>,
    pub f_assigned_reviewer_at: Option<Option<OffsetDateTime>>,
    pub f_assigned_publisher_at: Option<Option<OffsetDateTime>>,
    pub f_assigned_admin_at: Option<Option<OffsetDateTime>>,
    pub f_assigned_bot_at: Option<Option<OffsetDateTime>>,

    pub f_updated_at: OffsetDateTime,
}

impl<'a> MemberAspect<'a> {
    pub fn new(updated_at: OffsetDateTime) -> Self {
        Self {
            f_user_nickname: None,
            f_user_last_active_at: None,
            f_assigned_raw_provider_at: None,
            f_assigned_translator_at: None,
            f_assigned_proofreader_at: None,
            f_assigned_typesetter_at: None,
            f_assigned_redrawer_at: None,
            f_assigned_reviewer_at: None,
            f_assigned_publisher_at: None,
            f_assigned_admin_at: None,
            f_assigned_bot_at: None,
            f_updated_at: updated_at,
        }
    }

    pub fn user_nickname(mut self, val: &'a str) -> Self {
        //
        self.f_user_nickname = Some(val);

        self
    }

    pub fn user_last_active_at(mut self, val: OffsetDateTime) -> Self {
        //
        self.f_user_last_active_at = Some(val);

        self
    }

    pub fn assigned_raw_provider_at(
        mut self,
        val: Option<OffsetDateTime>,
    ) -> Self {
        //
        self.f_assigned_raw_provider_at = Some(val);

        self
    }

    pub fn assigned_translator_at(
        mut self,
        val: Option<OffsetDateTime>,
    ) -> Self {
        //
        self.f_assigned_translator_at = Some(val);

        self
    }

    pub fn assigned_proofreader_at(
        mut self,
        val: Option<OffsetDateTime>,
    ) -> Self {
        //
        self.f_assigned_proofreader_at = Some(val);

        self
    }

    pub fn assigned_typesetter_at(
        mut self,
        val: Option<OffsetDateTime>,
    ) -> Self {
        //
        self.f_assigned_typesetter_at = Some(val);

        self
    }

    pub fn assigned_redrawer_at(mut self, val: Option<OffsetDateTime>) -> Self {
        //
        self.f_assigned_redrawer_at = Some(val);

        self
    }

    pub fn assigned_reviewer_at(mut self, val: Option<OffsetDateTime>) -> Self {
        //
        self.f_assigned_reviewer_at = Some(val);

        self
    }

    pub fn assigned_publisher_at(
        mut self,
        val: Option<OffsetDateTime>,
    ) -> Self {
        //
        self.f_assigned_publisher_at = Some(val);

        self
    }

    pub fn assigned_admin_at(mut self, val: Option<OffsetDateTime>) -> Self {
        //
        self.f_assigned_admin_at = Some(val);

        self
    }

    pub fn assigned_bot_at(mut self, val: Option<OffsetDateTime>) -> Self {
        //
        self.f_assigned_bot_at = Some(val);

        self
    }
}

// ── Conversions ────────────────────────────────────────────────────────────

impl From<MemberRow> for MemberInfo {
    fn from(v: MemberRow) -> Self {
        //
        let mut bits: u32 = 0;

        if v.f_assigned_raw_provider_at.is_some() {
            bits |= u32::from(RoleField::RAW_PROVIDER);
        }

        if v.f_assigned_translator_at.is_some() {
            bits |= u32::from(RoleField::TRANSLATOR);
        }

        if v.f_assigned_proofreader_at.is_some() {
            bits |= u32::from(RoleField::PROOFREADER);
        }

        if v.f_assigned_typesetter_at.is_some() {
            bits |= u32::from(RoleField::TYPESETTER);
        }

        if v.f_assigned_redrawer_at.is_some() {
            bits |= u32::from(RoleField::REDRAWER);
        }

        if v.f_assigned_reviewer_at.is_some() {
            bits |= u32::from(RoleField::REVIEWER);
        }

        if v.f_assigned_publisher_at.is_some() {
            bits |= u32::from(RoleField::PUBLISHER);
        }

        if v.f_assigned_admin_at.is_some() {
            bits |= u32::from(RoleField::ADMIN);
        }

        let roles = RoleMask::try_from(bits)
            .unwrap_or_else(|_| RoleMask::from(RoleField::RAW_PROVIDER));

        MemberInfo {
            id: v.f_id,
            user_id: v.f_user_id,
            user_nickname: v.f_user_nickname,
            user_last_active_at: v.f_user_last_active_at,
            team_id: v.f_team_id,
            user: None,
            team: None,
            roles,
        }
    }
}
