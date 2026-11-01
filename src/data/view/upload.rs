//! Shared presigned-upload response views.

use std::collections::BTreeMap;

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// One header bound into a presigned upload request.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UploadHeaderView {
    /// Header name.
    pub name: String,

    /// Header value bound into the upload signature.
    pub value: String,
}

impl UploadHeaderView {
    /// Converts one bound header map into named entries.
    #[must_use]
    pub fn from_map(headers: BTreeMap<String, String>) -> Vec<Self> {
        //
        headers
            .into_iter()
            .map(|(name, value)| Self { name, value })
            .collect()
    }
}
