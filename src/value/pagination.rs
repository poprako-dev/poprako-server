//! Bounded pagination values for ordinary public lists.

#[cfg(test)]
mod tests;

use std::fmt::{Display, Formatter};

use serde::Deserialize;
use serde::de::Error as _;

/// Maximum number of records returned by one ordinary public list request.
pub const MAX_PUB_LIST_LIMIT: u32 = 200;

// Serde-facing error for a limit outside its compile-time range.
struct InvalidListLimit<const N: u32> {
    // The rejected raw limit value.
    limit: u32,
}

impl<const N: u32> Display for InvalidListLimit<N> {
    // Format the rejected limit error.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        //
        write!(
            formatter,
            "list limit {} is outside the inclusive range 1..={}",
            self.limit, N,
        )
    }
}

/// A list limit proven to be inside its compile-time maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListLimit<const N: u32 = 20>(u32);

impl<const N: u32> ListLimit<N> {
    /// Constructs a limit when it is inside `1..=N`.
    pub const fn new(limit: u32) -> Option<Self> {
        //
        match limit {
            //
            value if value > 0 && value <= N => Some(Self(value)),

            //
            _ => None,
        }
    }

    /// Returns the validated row count.
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl<'de, const N: u32> Deserialize<'de> for ListLimit<N> {
    // Deserialize and validate a list limit from an external input.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let limit = u32::deserialize(deserializer)?;

        Self::new(limit)
            .ok_or_else(|| D::Error::custom(InvalidListLimit::<N> { limit }))
    }
}

#[cfg(feature = "swagger")]
impl<const N: u32> utoipa::PartialSchema for ListLimit<N> {
    // Build the OpenAPI schema from the compile-time limit.
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        //
        use utoipa::openapi::schema::{
            KnownFormat, ObjectBuilder, SchemaFormat, Type,
        };

        ObjectBuilder::new()
            .schema_type(Type::Integer)
            .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int32)))
            .minimum(Some(1_u32))
            .maximum(Some(N))
            .into()
    }
}

#[cfg(feature = "swagger")]
impl<const N: u32> utoipa::ToSchema for ListLimit<N> {
    // Name the OpenAPI schema with its compile-time limit.
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Owned(format!("ListLimit{}", N))
    }
}

/// Limit used by ordinary public list endpoints.
pub type PubListLimit = ListLimit<MAX_PUB_LIST_LIMIT>;
