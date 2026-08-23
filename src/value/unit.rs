use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Text field selected by a Unit search or transform request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum UnitTextPart {
    //
    /// Current translated text.
    TranslatedText,

    /// Current proofread text.
    ProofreadText,
}

/// Content-field perms derived from the current assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitEditPerm {
    //
    /// Whether translation content may be changed.
    pub can_translate: bool,
    /// Whether revision content may be changed.
    pub can_proofread: bool,
}
