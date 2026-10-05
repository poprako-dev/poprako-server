//! Patch input shared by runtime deserialization and schema generation.
// Utoipa generic derive emits this lint in generated impls.
#![cfg_attr(
    feature = "swagger",
    allow(
        clippy::option_if_let_else,
        reason = "Generic serialization and schema derives emit this lint on non-Option fields"
    )
)]

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Tagged request representation shared by Patch deserialization and `OpenAPI`.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PatchInput<T> {
    /// Explicitly discard the stored value.
    Clear,

    /// Replace the stored value.
    Assign {
        /// Replacement value.
        value: T,
    },

    /// Preserve the stored value.
    Skip,
}
