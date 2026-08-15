//! Diesel entity types for the `t_team` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::read::proj::team::TeamInfo;
use crate::part_impl::repo::rdb_impl::schema::t_team;
use crate::result::{BaseError, BaseRest, accept};
use crate::value::image::{ImageExt, ImageHash};

// ── Queryable / Selectable ─────────────────────────────────────────────────

/// Raw database row for the `t_team` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_team)]
pub struct TeamInfoRow {
    pub f_id: String,
    pub f_name: String,
    pub f_description: Option<String>,

    pub f_avatar_key: Option<String>,
    pub f_avatar_uploaded: Option<bool>,
    pub f_avatar_version: Option<i64>,
    pub f_avatar_hash: Option<Vec<u8>>,
    pub f_avatar_extension: Option<String>,

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

    pub f_avatar_key: Option<&'a str>,
    pub f_avatar_uploaded: Option<bool>,
    pub f_avatar_version: Option<i64>,
    pub f_avatar_hash: Option<&'a [u8]>,
    pub f_avatar_extension: Option<&'a str>,

    pub f_updated_at: OffsetDateTime,
}

impl<'a> TeamAspectRow<'a> {
    pub fn new(updated_at: OffsetDateTime) -> Self {
        //
        Self {
            f_name: None,
            f_description: None,
            f_avatar_key: None,
            f_avatar_uploaded: None,
            f_avatar_version: None,
            f_avatar_hash: None,
            f_avatar_extension: None,
            f_updated_at: updated_at,
        }
    }

    pub fn name(mut self, val: &'a str) -> Self {
        //
        self.f_name = Some(val);

        self
    }

    pub fn description(mut self, val: &'a str) -> Self {
        //
        self.f_description = Some(val);

        self
    }

    pub fn avatar_key(mut self, val: &'a str) -> Self {
        //
        self.f_avatar_key = Some(val);

        self
    }

    pub fn avatar_uploaded(mut self, val: bool) -> Self {
        //
        self.f_avatar_uploaded = Some(val);

        self
    }

    pub fn avatar_version(mut self, val: u32) -> Self {
        //
        self.f_avatar_version = Some(i64::from(val));

        self
    }

    pub fn avatar_hash(mut self, val: &'a ImageHash) -> Self {
        //
        self.f_avatar_hash = Some(val.as_bytes());

        self
    }

    pub fn avatar_ext(mut self, val: ImageExt) -> Self {
        //
        self.f_avatar_extension = Some(val.suffix());

        self
    }
}

// ── Conversions ────────────────────────────────────────────────────────────

impl TryFrom<TeamInfoRow> for TeamInfo {
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    fn try_from(v: TeamInfoRow) -> BaseRest<Self> {
        //
        let (
            avatar_key,
            is_avatar_uploaded,
            avatar_version,
            avatar_hash,
            avatar_ext,
        ) = match (
            v.f_avatar_key,
            v.f_avatar_uploaded,
            v.f_avatar_version,
            v.f_avatar_hash,
            v.f_avatar_extension,
        ) {
            //
            (None, None, None, None, None) => (None, None, None, None, None),

            (
                Some(avatar_key),
                Some(is_avatar_uploaded),
                Some(avatar_version),
                Some(avatar_hash),
                Some(avatar_ext),
            ) => {
                //
                let avatar_version = u32::try_from(avatar_version).map_err(|_| {
                        //
                        BaseError::Unrecoverable {
                            message: "[TeamInfoRow] f_avatar_version must be non-negative".into(),
                        }
                    })?;

                let avatar_hash = avatar_hash.try_into().map_err(|_| {
                    //
                    BaseError::Unrecoverable {
                        message:
                            "[TeamInfoRow] f_avatar_hash must contain 32 bytes"
                                .into(),
                    }
                })?;

                let avatar_ext =
                    ImageExt::parse(&avatar_ext).ok_or_else(|| {
                        //
                        BaseError::Unrecoverable {
                        message:
                            "[TeamInfoRow] f_avatar_extension must be supported"
                                .into(),
                    }
                    })?;

                (
                    Some(avatar_key),
                    Some(is_avatar_uploaded),
                    Some(avatar_version),
                    Some(ImageHash::new(avatar_hash)),
                    Some(avatar_ext),
                )
            }

            _ => {
                //
                return Err(BaseError::Unrecoverable {
                        message: "[TeamInfoRow] avatar fields must be all null or all present".into(),
                    });
            }
        };

        accept(TeamInfo {
            id: v.f_id,
            name: v.f_name,
            description: v.f_description.unwrap_or_default(),
            avatar_key,
            is_avatar_uploaded,
            avatar_version,
            avatar_hash,
            avatar_ext,
            created_at: v.f_created_at,
            updated_at: v.f_updated_at,
        })
    }
}
