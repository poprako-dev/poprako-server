//! HTTP wire objects for query parameters.
use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::IntoParams;

use crate::value::pagination::PubListLimit;

/// Pagination query parameters for nested list endpoints where the parent id
/// is carried by the path.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct Pagination {
    /// Zero-based offset for paginated results.
    pub offset: u32,
    /// Maximum number of items to return.
    pub limit: PubListLimit,
}
