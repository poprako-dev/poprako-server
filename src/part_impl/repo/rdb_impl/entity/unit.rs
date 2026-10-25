#![allow(
    clippy::ref_option_ref,
    reason = "Diesel AsChangeset derives borrow nullable fields without changing their three-state update semantics"
)]

//! Diesel entity types for the `t_unit` table.
//!
//! FIXME: Diesel's generated `AsChangeset` implementation reports
//! `ref_option_ref` for the intentional tri-state `Option<Option<&T>>` fields.
//! Flattening them would change the unchanged, clear, and set update semantics.

use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use time::OffsetDateTime;

use crate::model::read::proj::unit::UnitInfo;
use crate::model::shared::unit::UnitCoord;
use crate::model::write::unit::UnitEdit;
use crate::part_impl::repo::rdb_impl::schema::t_unit;
use crate::util::Patch;

#[derive(Queryable, Selectable)]
#[diesel(table_name = t_unit)]
pub struct UnitInfoRecord {
    pub f_id: String,

    pub f_page_source_id: String,
    pub f_next_id: Option<String>,
    pub f_hidden_at: Option<OffsetDateTime>,

    pub f_is_bubble: bool,
    pub f_is_flagged: bool,
    pub f_is_proofread: bool,

    pub f_x_coord: f64,
    pub f_y_coord: f64,

    pub f_typography_note: Option<String>,
    pub f_translation_note: Option<String>,

    pub f_translated_text: Option<String>,
    pub f_last_translator_id: Option<String>,

    pub f_proofread_text: Option<String>,
    pub f_last_proofreader_id: Option<String>,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl From<UnitInfoRecord> for UnitInfo {
    fn from(record: UnitInfoRecord) -> Self {
        Self {
            id: record.f_id,
            page_source_id: record.f_page_source_id,
            next_id: record.f_next_id,
            is_bubble: record.f_is_bubble,
            is_flagged: record.f_is_flagged,
            coord: UnitCoord {
                x_coord: record.f_x_coord,
                y_coord: record.f_y_coord,
            },
            typography_note: record.f_typography_note,
            translation_note: record.f_translation_note,
            translated_text: record.f_translated_text,
            last_translator_id: record.f_last_translator_id,
            is_proofread: record.f_is_proofread,
            proofread_text: record.f_proofread_text,
            last_proofreader_id: record.f_last_proofreader_id,
            hidden_at: record.f_hidden_at,
            created_at: record.f_created_at,
            updated_at: record.f_updated_at,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = t_unit)]
pub struct UnitItem<'a> {
    pub f_id: &'a str,

    pub f_page_source_id: &'a str,
    pub f_next_id: Option<&'a str>,
    pub f_hidden_at: Option<OffsetDateTime>,

    pub f_is_bubble: bool,
    pub f_is_flagged: bool,
    pub f_is_proofread: bool,

    pub f_x_coord: f64,
    pub f_y_coord: f64,

    pub f_typography_note: Option<&'a str>,
    pub f_translation_note: Option<&'a str>,

    pub f_translated_text: Option<&'a str>,
    pub f_last_translator_id: Option<&'a str>,

    pub f_proofread_text: Option<&'a str>,
    pub f_last_proofreader_id: Option<&'a str>,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl<'a> UnitItem<'a> {
    #[must_use]
    pub fn from_edit(
        page_source_id: &'a str,
        edit: &'a UnitEdit,
        next_id: Option<&'a str>,
    ) -> Option<Self> {
        //
        let UnitEdit::Create {
            id,
            is_bubble,
            is_flagged,
            coord,
            typography_note,
            translation_note,
            translation,
            revision,
            ..
        } = edit
        else {
            return None;
        };

        let (translated_text, last_translator_id) =
            translation.as_ref().map_or((None, None), |translation| {
                //
                (
                    Some(translation.translated_text.as_str()),
                    Some(translation.last_translator_id.as_str()),
                )
            });

        let (is_proofread, proofread_text, last_proofreader_id) =
            revision.as_ref().map_or((false, None, None), |revision| {
                //
                (
                    revision.is_proofread,
                    revision.proofread_text.as_deref(),
                    Some(revision.last_proofreader_id.as_str()),
                )
            });

        let now = OffsetDateTime::now_utc();

        Some(Self {
            f_id: id,
            f_page_source_id: page_source_id,
            f_next_id: next_id,
            f_hidden_at: None,
            f_is_bubble: *is_bubble,
            f_is_flagged: *is_flagged,
            f_is_proofread: is_proofread,
            f_x_coord: coord.x_coord,
            f_y_coord: coord.y_coord,
            f_typography_note: typography_note.as_deref(),
            f_translation_note: translation_note.as_deref(),
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
pub struct UnitPack<'a> {
    pub f_next_id: Option<Option<&'a str>>,
    pub f_hidden_at: Option<Option<OffsetDateTime>>,

    pub f_is_bubble: Option<bool>,
    pub f_is_flagged: Option<bool>,
    pub f_is_proofread: Option<bool>,

    pub f_x_coord: Option<f64>,
    pub f_y_coord: Option<f64>,

    pub f_typography_note: Option<Option<&'a str>>,
    pub f_translation_note: Option<Option<&'a str>>,

    pub f_translated_text: Option<Option<&'a str>>,
    pub f_last_translator_id: Option<Option<&'a str>>,

    pub f_proofread_text: Option<Option<&'a str>>,
    pub f_last_proofreader_id: Option<Option<&'a str>>,

    pub f_updated_at: OffsetDateTime,
}

impl<'a> UnitPack<'a> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            f_next_id: None,
            f_hidden_at: None,
            f_is_bubble: None,
            f_is_flagged: None,
            f_is_proofread: None,
            f_x_coord: None,
            f_y_coord: None,
            f_typography_note: None,
            f_translation_note: None,
            f_translated_text: None,
            f_last_translator_id: None,
            f_proofread_text: None,
            f_last_proofreader_id: None,
            f_updated_at: OffsetDateTime::now_utc(),
        }
    }

    #[must_use]
    pub fn apply_edit(mut self, edit: &'a UnitEdit) -> Self {
        //
        let UnitEdit::Save {
            is_bubble,
            is_flagged,
            coord,
            typography_note,
            translation_note,
            translation,
            revision,
            ..
        } = edit
        else {
            return self;
        };

        self.f_is_bubble = *is_bubble;

        self.f_is_flagged = *is_flagged;

        if let Some(coord) = coord {
            //
            self.f_x_coord = Some(coord.x_coord);

            self.f_y_coord = Some(coord.y_coord);
        }

        self.f_typography_note = match typography_note {
            //
            Patch::Clear => Some(None),

            Patch::Assign { value } => Some(Some(value.as_str())),

            Patch::Skip => None,
        };

        self.f_translation_note = match translation_note {
            //
            Patch::Clear => Some(None),

            Patch::Assign { value } => Some(Some(value.as_str())),

            Patch::Skip => None,
        };

        match translation {
            //
            Patch::Clear => {
                //
                self.f_translated_text = Some(None);

                self.f_last_translator_id = Some(None);
            }

            Patch::Assign { value: translation } => {
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

            Patch::Assign { value: revision } => {
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

impl Default for UnitPack<'_> {
    fn default() -> Self {
        Self::new()
    }
}
