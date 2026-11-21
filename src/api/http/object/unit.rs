//! HTTP wire objects for the migrated data DTOs.
use std::ops::Not as _;

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::api::http::result::Wire;
use crate::data::instr::unit::{
    ListPageSourceUnitInfosInstr, SavePageSourceUnitEditsInstr,
    SearchChapterUnitInfosInstr, TransformChapterUnitsInstr, UnitCoordInstr,
    UnitEditInstr, UnitRevisionInstr, UnitTextTransformInstr,
    UnitTransformInstr, UnitTranslationInstr,
};
use crate::data::val::unit::{
    CreatedUnitIdVal, ListPageSourceUnitInfosVal, SavePageSourceUnitEditsVal,
};
use crate::data::view::unit::UnitInfoView;
use crate::util::Patch;

#[cfg(feature = "swagger")]
use crate::util::patch_input::PatchInput;

use crate::value::unit::UnitTextPart;

/// Input parameters for listing visible Units under one source page.
#[derive(Debug, Deserialize)]
pub struct ListPageSourceUnitInfosBody {
    /// source page whose visible Units are listed.
    pub page_source_id: String,
}

impl From<ListPageSourceUnitInfosBody> for ListPageSourceUnitInfosInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListPageSourceUnitInfosBody) -> Self {
        Self {
            page_source_id: value.page_source_id,
        }
    }
}

/// Input parameters for searching visible Units under one Chapter.
#[derive(Debug)]
pub struct SearchChapterUnitInfosBody {
    /// Chapter whose visible Units are searched.
    pub chapter_id: String,
    /// Unit text field selected for matching.
    pub part: UnitTextPart,
    /// Literal substring to search after normalization.
    pub phrase: String,
}

impl From<SearchChapterUnitInfosBody> for SearchChapterUnitInfosInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: SearchChapterUnitInfosBody) -> Self {
        Self {
            chapter_id: value.chapter_id,
            part: value.part,
            phrase: value.phrase,
        }
    }
}

/// Input parameters for saving a batch of Unit edits.
#[derive(Deserialize)]
pub struct SavePageSourceUnitEditsBody {
    /// Client-generated identity of this immutable save batch.
    pub save_id: String,

    /// source page whose Units are being edited.
    pub page_source_id: String,

    /// Ordered batch of Unit edits.
    pub edits: Vec<UnitEditBody>,
}

impl From<SavePageSourceUnitEditsBody> for SavePageSourceUnitEditsInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: SavePageSourceUnitEditsBody) -> Self {
        Self {
            save_id: value.save_id,
            page_source_id: value.page_source_id,
            edits: value
                .edits
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

/// One literal text transform supplied by the client.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(deny_unknown_fields)]
pub struct UnitTextTransformBody {
    /// Literal text selected in the original value.
    pub origin: String,
    /// Literal text inserted for each selected match.
    pub target: String,
}

impl From<UnitTextTransformBody> for UnitTextTransformInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UnitTextTransformBody) -> Self {
        Self {
            origin: value.origin,
            target: value.target,
        }
    }
}

/// Text transforms supplied for one permanent Unit.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(deny_unknown_fields)]
pub struct UnitTransformBody {
    /// Permanent target Unit ID.
    pub unit_id: String,
    /// Literal transforms evaluated against the same original text.
    pub transforms: Vec<UnitTextTransformBody>,
}

impl From<UnitTransformBody> for UnitTransformInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UnitTransformBody) -> Self {
        Self {
            unit_id: value.unit_id,
            transforms: value
                .transforms
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

/// Input parameters for transforming selected Units under one Chapter.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(deny_unknown_fields)]
pub struct TransformChapterUnitsBody {
    /// Unit text field selected for transformation.
    pub part: UnitTextPart,
    /// Selected Units and their literal transforms.
    pub units: Vec<UnitTransformBody>,
}

impl From<TransformChapterUnitsBody> for TransformChapterUnitsInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: TransformChapterUnitsBody) -> Self {
        Self {
            part: value.part,
            units: value
                .units
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

/// One transport-facing Unit edit.
#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(tag = "edit", rename_all = "snake_case", deny_unknown_fields)]
pub enum UnitEditBody {
    /// Creates one Unit with a request-local ID.
    Create {
        /// Request-local ID used for references within this batch.
        local_id: String,

        /// Unit before which the new Unit is inserted, or the tail when null.
        #[serde(default)]
        next_id: Option<String>,

        /// Whether the Unit identifies a speech bubble.
        is_bubble: bool,
        /// Initial review flag; omitted values are false and excluded from save digests.
        #[serde(default, skip_serializing_if = "<&bool>::not")]
        is_flagged: bool,
        /// Initial page-relative coordinate.
        coord: UnitCoordBody,

        /// Initial text annotation delivered to typesetters.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        typography_note: Option<String>,
        /// Initial communication between translators and proofreaders.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        translation_note: Option<String>,

        /// Optional initial translation assignment.
        #[serde(default)]
        translation: Option<UnitTranslationBody>,
        /// Optional initial revision assignment.
        #[serde(default)]
        revision: Option<UnitRevisionBody>,
    },

    /// Patches or restores one permanent Unit.
    Patch {
        /// Permanent target Unit ID.
        id: String,

        /// Three-state linked-list successor patch.
        #[serde(default)]
        #[cfg_attr(
            feature = "swagger",
            schema(value_type = Option<PatchInput<String>>)
        )]
        next_id: Patch<String>,

        /// Optional speech-bubble flag replacement.
        #[serde(default)]
        is_bubble: Option<bool>,
        /// Optional review flag replacement; null leaves it unchanged.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        is_flagged: Option<bool>,
        /// Optional coordinate replacement.
        #[serde(default)]
        coord: Option<UnitCoordBody>,

        /// Three-state typesetting annotation patch.
        #[serde(default, skip_serializing_if = "Patch::is_skip")]
        #[cfg_attr(
            feature = "swagger",
            schema(value_type = Option<PatchInput<String>>)
        )]
        typography_note: Patch<String>,
        /// Three-state translator and proofreader communication patch.
        #[serde(default, skip_serializing_if = "Patch::is_skip")]
        #[cfg_attr(
            feature = "swagger",
            schema(value_type = Option<PatchInput<String>>)
        )]
        translation_note: Patch<String>,

        /// Three-state translation patch.
        #[serde(default)]
        #[cfg_attr(
            feature = "swagger",
            schema(value_type = Option<PatchInput<UnitTranslationBody>>)
        )]
        translation: Patch<UnitTranslationBody>,
        /// Three-state revision patch.
        #[serde(default)]
        #[cfg_attr(
            feature = "swagger",
            schema(value_type = Option<PatchInput<UnitRevisionBody>>)
        )]
        revision: Patch<UnitRevisionBody>,
    },

    /// Soft-deletes one permanent Unit.
    Delete {
        /// Permanent target Unit ID.
        id: String,
    },
}

impl From<UnitEditBody> for UnitEditInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UnitEditBody) -> Self {
        //
        match value {
            //
            UnitEditBody::Create {
                local_id,
                next_id,
                is_bubble,
                is_flagged,
                coord,
                typography_note,
                translation_note,
                translation,
                revision,
            } => Self::Create {
                local_id,
                next_id,
                is_bubble,
                is_flagged,
                coord: coord.into(),
                typography_note,
                translation_note,
                translation: translation.map(std::convert::Into::into),
                revision: revision.map(std::convert::Into::into),
            },

            UnitEditBody::Patch {
                id,
                next_id,
                is_bubble,
                is_flagged,
                coord,
                typography_note,
                translation_note,
                translation,
                revision,
            } => Self::Patch {
                id,
                next_id,
                is_bubble,
                is_flagged,
                coord: coord.map(std::convert::Into::into),
                typography_note,
                translation_note,
                translation: translation.map(std::convert::Into::into),
                revision: revision.map(std::convert::Into::into),
            },

            UnitEditBody::Delete { id } => Self::Delete { id },
        }
    }
}

/// source page-relative Unit coordinates.
#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(deny_unknown_fields)]
pub struct UnitCoordBody {
    /// Horizontal page-relative coordinate.
    pub x_coord: f64,
    /// Vertical page-relative coordinate.
    pub y_coord: f64,
}

impl From<UnitCoordBody> for UnitCoordInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UnitCoordBody) -> Self {
        Self {
            x_coord: value.x_coord,
            y_coord: value.y_coord,
        }
    }
}

/// Translation assignment accepted from the client.
#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(deny_unknown_fields)]
pub struct UnitTranslationBody {
    /// Replacement translated text.
    pub translated_text: String,
}

impl From<UnitTranslationBody> for UnitTranslationInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UnitTranslationBody) -> Self {
        Self {
            translated_text: value.translated_text,
        }
    }
}

/// Revision assignment accepted from the client.
#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(deny_unknown_fields)]
pub struct UnitRevisionBody {
    /// Replacement approval state.
    pub is_proofread: bool,

    /// Replacement proofread text.
    #[serde(default)]
    pub proofread_text: Option<String>,
}

impl From<UnitRevisionBody> for UnitRevisionInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UnitRevisionBody) -> Self {
        Self {
            is_proofread: value.is_proofread,
            proofread_text: value.proofread_text,
        }
    }
}

/// Return value for listing visible Units under one source page.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ListPageSourceUnitInfosPayload {
    /// Visible Units in final linked-list order.
    pub unit_infos: Vec<UnitInfoObj>,

    /// Number of visible Units.
    pub total_unit_count: usize,
    /// Number of visible translated Units.
    pub translated_unit_count: usize,
    /// Number of visible proofread Units.
    pub proofread_unit_count: usize,
}

impl From<ListPageSourceUnitInfosVal> for ListPageSourceUnitInfosPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListPageSourceUnitInfosVal) -> Self {
        Self {
            unit_infos: value
                .unit_infos
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
            total_unit_count: value.total_unit_count,
            translated_unit_count: value.translated_unit_count,
            proofread_unit_count: value.proofread_unit_count,
        }
    }
}

impl Wire for ListPageSourceUnitInfosVal {
    // Serializable wire payload produced by this conversion.
    type Target = ListPageSourceUnitInfosPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// One explicit identity pair returned for a newly created Unit.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreatedUnitIdPayload {
    /// Client identity from the create operation.
    pub local_id: String,
    /// Permanent Unit identity.
    pub unit_id: String,
}

impl From<CreatedUnitIdVal> for CreatedUnitIdPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreatedUnitIdVal) -> Self {
        Self {
            local_id: value.local_id,
            unit_id: value.unit_id,
        }
    }
}

impl Wire for CreatedUnitIdVal {
    // Serializable wire payload produced by this conversion.
    type Target = CreatedUnitIdPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Result of every successful Unit save, including replayed saves.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct SavePageSourceUnitEditsPayload {
    /// Explicit identity pairs; empty when the batch creates no Units.
    pub created_unit_ids: Vec<CreatedUnitIdPayload>,
}

impl From<SavePageSourceUnitEditsVal> for SavePageSourceUnitEditsPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: SavePageSourceUnitEditsVal) -> Self {
        Self {
            created_unit_ids: value
                .created_unit_ids
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

impl Wire for SavePageSourceUnitEditsVal {
    // Serializable wire payload produced by this conversion.
    type Target = SavePageSourceUnitEditsPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Presentation-ready visible Unit information.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UnitInfoObj {
    /// Permanent Unit ID.
    pub id: String,

    /// Owning source page ID.
    pub page_source_id: String,

    /// Whether the Unit identifies a speech bubble.
    pub is_bubble: bool,
    /// Whether this Unit is flagged for later review.
    pub is_flagged: bool,
    /// Whether the current revision is approved.
    pub is_proofread: bool,

    /// Horizontal page-relative coordinate.
    pub x_coord: f64,
    /// Vertical page-relative coordinate.
    pub y_coord: f64,

    /// Translation text annotation delivered to typesetters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typography_note: Option<String>,
    /// Communication between translators and proofreaders.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation_note: Option<String>,

    /// Current translated text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translated_text: Option<String>,
    /// ID of the translator who last assigned translation content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_translator_id: Option<String>,

    /// Current proofread text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proofread_text: Option<String>,
    /// ID of the proofreader who last assigned revision content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_proofreader_id: Option<String>,

    /// Creation time as Unix milliseconds.
    pub created_at: i64,
    /// Last update time as Unix milliseconds.
    pub updated_at: i64,
}

impl From<UnitInfoView> for UnitInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UnitInfoView) -> Self {
        Self {
            id: value.id,
            page_source_id: value.page_source_id,
            is_bubble: value.is_bubble,
            is_flagged: value.is_flagged,
            is_proofread: value.is_proofread,
            x_coord: value.x_coord,
            y_coord: value.y_coord,
            typography_note: value.typography_note,
            translation_note: value.translation_note,
            translated_text: value.translated_text,
            last_translator_id: value.last_translator_id,
            proofread_text: value.proofread_text,
            last_proofreader_id: value.last_proofreader_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl Wire for UnitInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = UnitInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Query parameters for Chapter Unit text searches.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
#[serde(deny_unknown_fields)]
pub struct UnitSearchQuery {
    /// Unit text field selected for matching.
    pub part: UnitTextPart,
    /// Case-sensitive literal substring to search.
    pub phrase: String,
}

/// Identity of one immutable Unit save batch.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
#[serde(deny_unknown_fields)]
pub struct UnitSaveQuery {
    /// Client-generated save UUID, reused only for the identical batch.
    pub save_id: String,
}
