//! Diesel entity types for the `t_team` table.

use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use time::OffsetDateTime;

use crate::model::read::proj::team::TeamInfo;
use crate::part_impl::repo::rdb_impl::schema::t_team;
use crate::result::{BaseError, BaseRest, accept};

// ── Queryable / Selectable ─────────────────────────────────────────────────

/// Raw database row for the `t_team` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_team)]
pub struct TeamInfoRow {
    pub f_id: String,
    pub f_name: String,
    pub f_description: Option<String>,

    pub f_workset_next_index: i32,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Insertable ─────────────────────────────────────────────────────────────

/// Insertable struct for creating a new record in the `t_team` table.
#[derive(Insertable)]
#[diesel(table_name = t_team)]
pub struct TeamEntryRow<'a> {
    pub f_id: &'a str,
    pub f_name: &'a str,
    pub f_description: &'a str,

    pub f_workset_next_index: i32,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Changeset (AsChangeset) ────────────────────────────────────────────────

/// Aspect struct for updating specific fields of a team record identified by id.
#[derive(AsChangeset)]
#[diesel(table_name = t_team)]
pub struct TeamAspectRow<'a> {
    pub f_name: Option<&'a str>,
    pub f_description: Option<&'a str>,

    pub f_updated_at: OffsetDateTime,
}

impl<'a> TeamAspectRow<'a> {
    pub const fn new(updated_at: OffsetDateTime) -> Self {
        //
        Self {
            f_name: None,
            f_description: None,
            f_updated_at: updated_at,
        }
    }

    pub const fn name(mut self, val: &'a str) -> Self {
        //
        self.f_name = Some(val);

        self
    }

    pub const fn description(mut self, val: &'a str) -> Self {
        //
        self.f_description = Some(val);

        self
    }
}

// ── Conversions ────────────────────────────────────────────────────────────

impl TryFrom<TeamInfoRow> for TeamInfo {
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    fn try_from(v: TeamInfoRow) -> BaseRest<Self> {
        //
        accept(Self {
            id: v.f_id,
            name: v.f_name,
            description: v.f_description.unwrap_or_default(),
            created_at: v.f_created_at,
            updated_at: v.f_updated_at,
        })
    }
}
