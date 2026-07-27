//! Diesel entity types for the `t_unit` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::read::proj::unit::UnitInfo;
use crate::model::shared::unit::UnitCoord;
use crate::model::write::unit::UnitEdit;
use crate::part_impl::repo::rdb_impl::schema::t_unit;
use crate::util::Patch;

#[derive(Queryable, Selectable)]
#[diesel(table_name = t_unit)]
pub struct UnitRow {
    //
    pub f_id: String,

    pub f_page_id: String,
    pub f_next_id: Option<String>,
    pub f_hidden_at: Option<OffsetDateTime>,

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

impl From<UnitRow> for UnitInfo {
    fn from(row: UnitRow) -> Self {
        Self {
            id: row.f_id,
            page_id: row.f_page_id,
            next_id: row.f_next_id,
            is_bubble: row.f_is_bubble,
            coord: UnitCoord {
                x_coord: row.f_x_coord,
                y_coord: row.f_y_coord,
            },
            translated_text: row.f_translated_text,
            last_translator_id: row.f_last_translator_id,
            is_proofread: row.f_is_proofread,
            proofread_text: row.f_proofread_text,
            last_proofreader_id: row.f_last_proofreader_id,
            hidden_at: row.f_hidden_at,
            created_at: row.f_created_at,
            updated_at: row.f_updated_at,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = t_unit)]
pub struct UnitEntry<'a> {
    //
    pub f_id: &'a str,

    pub f_page_id: &'a str,
    pub f_next_id: Option<&'a str>,
    pub f_hidden_at: Option<OffsetDateTime>,

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

impl<'a> UnitEntry<'a> {
    pub fn from_edit(page_id: &'a str, edit: &'a UnitEdit) -> Option<Self> {
        //
        let UnitEdit::Create {
            id,
            is_bubble,
            coord,
            translation,
            revision,
            ..
        } = edit
        else {
            return None;
        };

        let (translated_text, last_translator_id) = match translation {
            //
            Some(translation) => (
                Some(translation.translated_text.as_str()),
                Some(translation.last_translator_id.as_str()),
            ),

            None => (None, None),
        };

        let (is_proofread, proofread_text, last_proofreader_id) = match revision
        {
            Some(revision) => (
                revision.is_proofread,
                revision.proofread_text.as_deref(),
                Some(revision.last_proofreader_id.as_str()),
            ),

            None => (false, None, None),
        };

        let now = OffsetDateTime::now_utc();

        Some(Self {
            f_id: id,
            f_page_id: page_id,
            f_next_id: None,
            f_hidden_at: None,
            f_is_bubble: *is_bubble,
            f_is_proofread: is_proofread,
            f_x_coord: coord.x_coord,
            f_y_coord: coord.y_coord,
            f_translated_text: translated_text,
            f_last_translator_id: last_translator_id,
            f_proofread_text: proofread_text,
            f_last_proofreader_id: last_proofreader_id,
            f_created_at: now,
            f_updated_at: now,
        })
    }
}

#[derive(AsChangeset)]
#[diesel(table_name = t_unit)]
pub struct UnitAspect<'a> {
    //
    pub f_next_id: Option<Option<&'a str>>,
    pub f_hidden_at: Option<Option<OffsetDateTime>>,

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
    pub fn new() -> Self {
        Self {
            f_next_id: None,
            f_hidden_at: None,
            f_is_bubble: None,
            f_is_proofread: None,
            f_x_coord: None,
            f_y_coord: None,
            f_translated_text: None,
            f_last_translator_id: None,
            f_proofread_text: None,
            f_last_proofreader_id: None,
            f_updated_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn order(mut self, next_id: Option<&'a str>) -> Self {
        //
        self.f_next_id = Some(next_id);

        self
    }

    pub fn hide(mut self) -> Self {
        //
        self.f_hidden_at = Some(Some(OffsetDateTime::now_utc()));

        self
    }

    pub fn apply_edit(mut self, edit: &'a UnitEdit) -> Self {
        //
        let UnitEdit::Save {
            is_bubble,
            coord,
            translation,
            revision,
            ..
        } = edit
        else {
            return self;
        };

        self.f_hidden_at = Some(None);

        self.f_is_bubble = *is_bubble;

        if let Some(coord) = coord {
            //
            self.f_x_coord = Some(coord.x_coord);

            self.f_y_coord = Some(coord.y_coord);
        }

        match translation {
            //
            Patch::Clear => {
                //
                self.f_translated_text = Some(None);

                self.f_last_translator_id = Some(None);
            }

            Patch::Assign(translation) => {
                //
                self.f_translated_text =
                    Some(Some(translation.translated_text.as_str()));

                self.f_last_translator_id =
                    Some(Some(translation.last_translator_id.as_str()));
            }

            Patch::Skip => {}
        }

        match revision {
            //
            Patch::Clear => {
                //
                self.f_is_proofread = Some(false);

                self.f_proofread_text = Some(None);

                self.f_last_proofreader_id = Some(None);
            }

            Patch::Assign(revision) => {
                //
                self.f_is_proofread = Some(revision.is_proofread);

                self.f_proofread_text =
                    Some(revision.proofread_text.as_deref());

                self.f_last_proofreader_id =
                    Some(Some(revision.last_proofreader_id.as_str()));
            }

            Patch::Skip => {}
        }

        self
    }
}
