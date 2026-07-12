//! Diesel entity types for the `t_unit` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::unit_model;
use crate::part_impl::repo::rdb_impl::schema::t_unit;

/// Raw database row for the `t_unit` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_unit)]
pub struct UnitRow {
    pub f_id: String,

    pub f_page_id: String,
    pub f_index: i32,

    pub f_is_bubble: bool,
    pub f_is_proofread: bool,

    pub f_x_coord: f64,
    pub f_y_coord: f64,

    pub f_translated_text: Option<String>,
    pub f_last_translator_id: Option<String>,

    pub f_proofread_text: Option<String>,
    pub f_last_proofreader_id: Option<String>,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

/// Insertable struct for creating a new record in the `t_unit` table.
#[derive(Insertable)]
#[diesel(table_name = t_unit)]
pub struct UnitEntry<'a> {
    pub f_id: &'a str,

    pub f_page_id: &'a str,
    pub f_index: i32,

    pub f_is_bubble: bool,
    pub f_is_proofread: bool,

    pub f_x_coord: f64,
    pub f_y_coord: f64,

    pub f_translated_text: Option<&'a str>,
    pub f_last_translator_id: Option<&'a str>,

    pub f_proofread_text: Option<&'a str>,
    pub f_last_proofreader_id: Option<&'a str>,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

/// Aspect struct for updating specific fields of a unit record identified by id.
#[derive(AsChangeset)]
#[diesel(table_name = t_unit)]
pub struct UnitAspect<'a> {
    pub f_index: Option<i32>,

    pub f_is_bubble: Option<bool>,
    pub f_is_proofread: Option<bool>,

    pub f_x_coord: Option<f64>,
    pub f_y_coord: Option<f64>,

    pub f_translated_text: Option<Option<&'a str>>,
    pub f_last_translator_id: Option<Option<&'a str>>,

    pub f_proofread_text: Option<Option<&'a str>>,
    pub f_last_proofreader_id: Option<Option<&'a str>>,

    pub f_updated_at: OffsetDateTime,
}

impl<'a> UnitAspect<'a> {
    pub fn new(updated_at: OffsetDateTime) -> Self {
        Self {
            f_index: None,
            f_is_bubble: None,
            f_is_proofread: None,
            f_x_coord: None,
            f_y_coord: None,
            f_translated_text: None,
            f_last_translator_id: None,
            f_proofread_text: None,
            f_last_proofreader_id: None,
            f_updated_at: updated_at,
        }
    }

    pub fn index(mut self, val: i32) -> Self {
        //
        self.f_index = Some(val);

        self
    }

    pub fn payload(mut self, payload: &'a unit_model::Payload) -> Self {
        //
        self.f_is_bubble = Some(payload.is_bubble);

        self.f_is_proofread = Some(payload.is_proofread);

        self.f_x_coord = Some(payload.x_coord);

        self.f_y_coord = Some(payload.y_coord);

        self.f_translated_text = Some(payload.translated_text.as_deref());

        self.f_last_translator_id = Some(payload.last_translator_id.as_deref());

        self.f_proofread_text = Some(payload.proofread_text.as_deref());

        self.f_last_proofreader_id =
            Some(payload.last_proofreader_id.as_deref());

        self
    }
}

impl From<UnitRow> for unit_model::Info {
    fn from(row: UnitRow) -> Self {
        Self {
            id: row.f_id,
            page_id: row.f_page_id,
            index: row.f_index,
            is_bubble: row.f_is_bubble,
            is_proofread: row.f_is_proofread,
            x_coord: row.f_x_coord,
            y_coord: row.f_y_coord,
            translated_text: row.f_translated_text,
            last_translator_id: row.f_last_translator_id,
            proofread_text: row.f_proofread_text,
            last_proofreader_id: row.f_last_proofreader_id,
            created_at: row.f_created_at,
            updated_at: row.f_updated_at,
        }
    }
}

impl<'a> UnitEntry<'a> {
    pub fn new(
        id: &'a str,
        page_id: &'a str,
        index: i32,
        payload: &'a unit_model::Payload,
    ) -> Self {
        //
        let now = OffsetDateTime::now_utc();

        Self {
            f_id: id,
            f_page_id: page_id,
            f_index: index,
            f_is_bubble: payload.is_bubble,
            f_is_proofread: payload.is_proofread,
            f_x_coord: payload.x_coord,
            f_y_coord: payload.y_coord,
            f_translated_text: payload.translated_text.as_deref(),
            f_last_translator_id: payload.last_translator_id.as_deref(),
            f_proofread_text: payload.proofread_text.as_deref(),
            f_last_proofreader_id: payload.last_proofreader_id.as_deref(),
            f_created_at: now,
            f_updated_at: now,
        }
    }
}
