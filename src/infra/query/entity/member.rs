use diesel::prelude::*;
use time::OffsetDateTime;

use crate::domain::model::aggr::member::MemberAggr;
use crate::infra::query::schema;

// ── Queryable / Selectable ─────────────────────────────────────────────────

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::t_member)]
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
    pub f_assigned_assistant_at: Option<OffsetDateTime>,
    pub f_user_last_active_at: OffsetDateTime,
    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Insertable ─────────────────────────────────────────────────────────────

#[derive(Insertable)]
#[diesel(table_name = schema::t_member)]
pub struct MemberEntry<'a> {
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
    pub f_assigned_assistant_at: Option<OffsetDateTime>,
    pub f_user_last_active_at: OffsetDateTime,
    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Changeset (AsChangeset) ────────────────────────────────────────────────

/// Changeset for updating member fields via partial updates.
///
/// Only `Some` fields are included in the generated `SET` clause;
/// `None` fields are omitted.
#[derive(AsChangeset)]
#[diesel(table_name = schema::t_member)]
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
    pub f_assigned_assistant_at: Option<Option<OffsetDateTime>>,
    pub f_updated_at: OffsetDateTime,
}

impl<'a> MemberAspect<'a> {
    /// Creates a new changeset with all optional fields set to `None`.
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
            f_assigned_assistant_at: None,
            f_updated_at: updated_at,
        }
    }

    pub fn user_nickname(mut self, val: &'a str) -> Self {
        self.f_user_nickname = Some(val);
        self
    }

    pub fn user_last_active_at(mut self, val: OffsetDateTime) -> Self {
        self.f_user_last_active_at = Some(val);
        self
    }

    pub fn assigned_raw_provider_at(mut self, val: Option<OffsetDateTime>) -> Self {
        self.f_assigned_raw_provider_at = Some(val);
        self
    }

    pub fn assigned_translator_at(mut self, val: Option<OffsetDateTime>) -> Self {
        self.f_assigned_translator_at = Some(val);
        self
    }

    pub fn assigned_proofreader_at(mut self, val: Option<OffsetDateTime>) -> Self {
        self.f_assigned_proofreader_at = Some(val);
        self
    }

    pub fn assigned_typesetter_at(mut self, val: Option<OffsetDateTime>) -> Self {
        self.f_assigned_typesetter_at = Some(val);
        self
    }

    pub fn assigned_redrawer_at(mut self, val: Option<OffsetDateTime>) -> Self {
        self.f_assigned_redrawer_at = Some(val);
        self
    }

    pub fn assigned_reviewer_at(mut self, val: Option<OffsetDateTime>) -> Self {
        self.f_assigned_reviewer_at = Some(val);
        self
    }

    pub fn assigned_publisher_at(mut self, val: Option<OffsetDateTime>) -> Self {
        self.f_assigned_publisher_at = Some(val);
        self
    }

    pub fn assigned_admin_at(mut self, val: Option<OffsetDateTime>) -> Self {
        self.f_assigned_admin_at = Some(val);
        self
    }

    pub fn assigned_assistant_at(mut self, val: Option<OffsetDateTime>) -> Self {
        self.f_assigned_assistant_at = Some(val);
        self
    }
}

// ── Conversions ────────────────────────────────────────────────────────────

impl From<MemberRow> for MemberAggr {
    fn from(v: MemberRow) -> Self {
        MemberAggr {
            id: v.f_id,
            user_id: v.f_user_id,
            user_nickname: v.f_user_nickname,
            user: None,
            team_id: v.f_team_id,
            team: None,
            assigned_raw_provider_at: v.f_assigned_raw_provider_at,
            assigned_translator_at: v.f_assigned_translator_at,
            assigned_proofreader_at: v.f_assigned_proofreader_at,
            assigned_typesetter_at: v.f_assigned_typesetter_at,
            assigned_redrawer_at: v.f_assigned_redrawer_at,
            assigned_reviewer_at: v.f_assigned_reviewer_at,
            assigned_publisher_at: v.f_assigned_publisher_at,
            assigned_admin_at: v.f_assigned_admin_at,
            assigned_assistant_at: v.f_assigned_assistant_at,
            user_last_active_at: v.f_user_last_active_at,
            created_at: v.f_created_at,
            updated_at: v.f_updated_at,
        }
    }
}
