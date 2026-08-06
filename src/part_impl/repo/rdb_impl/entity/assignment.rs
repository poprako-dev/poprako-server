//! Diesel entity types for the `t_assignment` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::write::assignment::AssignmentEntry;
use crate::part_impl::repo::rdb_impl::schema::t_assignment;
use crate::result::BaseError;
use crate::value::role::{RoleField, RoleMask};

/// Raw database row for the `t_assignment` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_assignment)]
pub struct AssignmentInfoRow {
    //
    pub f_id: String,
    pub f_chapter_id: String,
    pub f_user_id: String,

    pub f_assigned_raw_provider_at: Option<OffsetDateTime>,
    pub f_assigned_translator_at: Option<OffsetDateTime>,
    pub f_assigned_proofreader_at: Option<OffsetDateTime>,
    pub f_assigned_typesetter_at: Option<OffsetDateTime>,
    pub f_assigned_redrawer_at: Option<OffsetDateTime>,
    pub f_assigned_reviewer_at: Option<OffsetDateTime>,
    pub f_assigned_publisher_at: Option<OffsetDateTime>,
    pub f_assigned_admin_at: Option<OffsetDateTime>,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl TryFrom<AssignmentInfoRow> for AssignmentInfo {
    type Error = BaseError;

    fn try_from(row: AssignmentInfoRow) -> Result<Self, Self::Error> {
        //
        let mut bits = 0;

        if row.f_assigned_raw_provider_at.is_some() {
            bits |= u32::from(RoleField::RAW_PROVIDER);
        }

        if row.f_assigned_translator_at.is_some() {
            bits |= u32::from(RoleField::TRANSLATOR);
        }

        if row.f_assigned_proofreader_at.is_some() {
            bits |= u32::from(RoleField::PROOFREADER);
        }

        if row.f_assigned_typesetter_at.is_some() {
            bits |= u32::from(RoleField::TYPESETTER);
        }

        if row.f_assigned_redrawer_at.is_some() {
            bits |= u32::from(RoleField::REDRAWER);
        }

        if row.f_assigned_reviewer_at.is_some() {
            bits |= u32::from(RoleField::REVIEWER);
        }

        if row.f_assigned_publisher_at.is_some() {
            bits |= u32::from(RoleField::PUBLISHER);
        }

        if row.f_assigned_admin_at.is_some() {
            bits |= u32::from(RoleField::ADMIN);
        }

        let roles = RoleMask::try_from(bits)?;

        Ok(Self {
            id: row.f_id,
            chapter_id: row.f_chapter_id,
            user_id: row.f_user_id,
            user: None,
            chapter: None,
            roles,
            created_at: row.f_created_at,
            updated_at: row.f_updated_at,
        })
    }
}

/// Insertable struct for creating a new record in the `t_assignment` table.
#[derive(Insertable)]
#[diesel(table_name = t_assignment)]
pub struct AssignmentEntryRow<'a> {
    //
    pub f_id: &'a str,
    pub f_chapter_id: &'a str,
    pub f_user_id: &'a str,

    pub f_assigned_raw_provider_at: Option<OffsetDateTime>,
    pub f_assigned_translator_at: Option<OffsetDateTime>,
    pub f_assigned_proofreader_at: Option<OffsetDateTime>,
    pub f_assigned_typesetter_at: Option<OffsetDateTime>,
    pub f_assigned_redrawer_at: Option<OffsetDateTime>,
    pub f_assigned_reviewer_at: Option<OffsetDateTime>,
    pub f_assigned_publisher_at: Option<OffsetDateTime>,
    pub f_assigned_admin_at: Option<OffsetDateTime>,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl<'a> AssignmentEntryRow<'a> {
    pub fn from_model_entry(
        model_entry: &'a AssignmentEntry,
        now: OffsetDateTime,
    ) -> Self {
        //
        let timestamps =
            AssignmentRoleTimestamps::from_mask(model_entry.roles, now);

        Self {
            f_id: &model_entry.id,
            f_chapter_id: &model_entry.chapter_id,
            f_user_id: &model_entry.user_id,
            f_assigned_raw_provider_at: timestamps.f_raw_provider,
            f_assigned_translator_at: timestamps.f_translator,
            f_assigned_proofreader_at: timestamps.f_proofreader,
            f_assigned_typesetter_at: timestamps.f_typesetter,
            f_assigned_redrawer_at: timestamps.f_redrawer,
            f_assigned_reviewer_at: timestamps.f_reviewer,
            f_assigned_publisher_at: timestamps.f_publisher,
            f_assigned_admin_at: timestamps.f_admin,
            f_created_at: now,
            f_updated_at: now,
        }
    }
}

/// Aspect struct for updating specific assignment role-timestamp fields by id.
#[derive(AsChangeset)]
#[diesel(table_name = t_assignment)]
pub struct AssignmentAspectRow {
    //
    pub f_assigned_raw_provider_at: Option<Option<OffsetDateTime>>,
    pub f_assigned_translator_at: Option<Option<OffsetDateTime>>,
    pub f_assigned_proofreader_at: Option<Option<OffsetDateTime>>,
    pub f_assigned_typesetter_at: Option<Option<OffsetDateTime>>,
    pub f_assigned_redrawer_at: Option<Option<OffsetDateTime>>,
    pub f_assigned_reviewer_at: Option<Option<OffsetDateTime>>,
    pub f_assigned_publisher_at: Option<Option<OffsetDateTime>>,
    pub f_assigned_admin_at: Option<Option<OffsetDateTime>>,

    pub f_updated_at: OffsetDateTime,
}

impl AssignmentAspectRow {
    pub fn new(updated_at: OffsetDateTime) -> Self {
        //
        Self {
            f_assigned_raw_provider_at: None,
            f_assigned_translator_at: None,
            f_assigned_proofreader_at: None,
            f_assigned_typesetter_at: None,
            f_assigned_redrawer_at: None,
            f_assigned_reviewer_at: None,
            f_assigned_publisher_at: None,
            f_assigned_admin_at: None,
            f_updated_at: updated_at,
        }
    }

    pub fn roles(mut self, timestamps: AssignmentRoleTimestamps) -> Self {
        //
        self.f_assigned_raw_provider_at = Some(timestamps.f_raw_provider);

        self.f_assigned_translator_at = Some(timestamps.f_translator);

        self.f_assigned_proofreader_at = Some(timestamps.f_proofreader);

        self.f_assigned_typesetter_at = Some(timestamps.f_typesetter);

        self.f_assigned_redrawer_at = Some(timestamps.f_redrawer);

        self.f_assigned_reviewer_at = Some(timestamps.f_reviewer);

        self.f_assigned_publisher_at = Some(timestamps.f_publisher);

        self.f_assigned_admin_at = Some(timestamps.f_admin);

        self
    }
}

/// Timestamps for each role on an assignment, used to build the role-timestamp
/// mapping for `AssignmentAspectRow` or `AssignmentEntry`.
pub struct AssignmentRoleTimestamps {
    //
    pub f_raw_provider: Option<OffsetDateTime>,
    pub f_translator: Option<OffsetDateTime>,
    pub f_proofreader: Option<OffsetDateTime>,
    pub f_typesetter: Option<OffsetDateTime>,
    pub f_redrawer: Option<OffsetDateTime>,
    pub f_reviewer: Option<OffsetDateTime>,
    pub f_publisher: Option<OffsetDateTime>,
    pub f_admin: Option<OffsetDateTime>,
}

impl AssignmentRoleTimestamps {
    pub fn from_mask(roles: RoleMask, now: OffsetDateTime) -> Self {
        //
        let timestamp_fn = |field: RoleField| -> Option<OffsetDateTime> {
            roles.has_any_role(&[field]).then_some(now)
        };

        Self {
            f_raw_provider: timestamp_fn(RoleField::RAW_PROVIDER),
            f_translator: timestamp_fn(RoleField::TRANSLATOR),
            f_proofreader: timestamp_fn(RoleField::PROOFREADER),
            f_typesetter: timestamp_fn(RoleField::TYPESETTER),
            f_redrawer: timestamp_fn(RoleField::REDRAWER),
            f_reviewer: timestamp_fn(RoleField::REVIEWER),
            f_publisher: timestamp_fn(RoleField::PUBLISHER),
            f_admin: timestamp_fn(RoleField::ADMIN),
        }
    }
}
