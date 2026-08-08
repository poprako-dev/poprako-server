//! Reusable traits and helpers shared across domain modules.
//!
//! This module currently hosts common value-object helpers that are used by
//! many layers (model conversion, storage identifiers, and partial updates).

use std::sync::OnceLock;

use serde::{Deserialize, Deserializer};

#[cfg(test)]
mod tests;

/// Generate a unique time-ordered identifier in base62 format.
///
/// A u64 fits in 11 base62 characters (vs 16 hex chars).
pub fn next_snowflake_id() -> String {
    format!("{:0>11}", base62::encode(next_snowflake_u64()))
}

/// Generate a unique time-ordered 64-bit value backed by a snowflake.
pub fn next_snowflake_u64() -> u64 {
    //
    // Make sure snowflake instance is initialized yet.
    ensure_snowflake_init();

    // Generate a snowflake id.
    k_snowflake::create_snowflake().to_decimal() as u64
}

// Initialise the global snowflake instance once from the
// `POPRAKO_SNOWFLAKE_NODE_ID` env var (defaults to 0).
fn ensure_snowflake_init() {
    //
    // Only init snowflake instance once.
    static INIT_GUARD: OnceLock<()> = OnceLock::new();

    INIT_GUARD
        .get_or_init(|| k_snowflake::set_instance(load_snowflake_node_id()));
}

// Load the snowflake node ID from the `POPRAKO_SNOWFLAKE_NODE_ID` env var.
fn load_snowflake_node_id() -> u16 {
    //
    let Ok(value) = std::env::var("POPRAKO_SNOWFLAKE_NODE_ID") else {
        return 0;
    };

    match value.parse() {
        //
        Ok(instance) if instance <= 1023 => instance,

        _ => unreachable!(),
    }
}

/// One transport-independent patch field.
///
/// `Skip` preserves the stored value, `Clear` resets it, and `Assign` replaces
/// it with the carried value.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Patch<T> {
    //
    /// Resets the stored field.
    Clear,

    /// Replaces the stored field with the carried value.
    Assign(T),

    /// Preserves the stored field.
    #[default]
    Skip,
}

impl<T> Patch<T> {
    /// Maps an assigned value while preserving Clear and Skip.
    pub fn map<U, F>(self, assign: F) -> Patch<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            //
            Self::Clear => Patch::Clear,

            Self::Assign(value) => Patch::Assign(assign(value)),

            Self::Skip => Patch::Skip,
        }
    }

    /// Reports whether this patch leaves the stored value unchanged.
    pub const fn is_skip(&self) -> bool {
        matches!(self, Self::Skip)
    }
}

impl<'de, T> Deserialize<'de> for Patch<T>
where
    T: Deserialize<'de>,
{
    // Deserializes null to Skip and requires an explicit tagged patch value.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Discriminant-only tagged representation for Patch deserialization.
        #[derive(Deserialize)]
        #[serde(tag = "type", content = "value", rename_all = "snake_case")]
        enum PatchVal<T> {
            //
            /// Explicit clear — discard the current value.
            Clear,

            /// Explicit assign — replace with the given value.
            Assign(T),

            /// Explicit skip or absent — leave the current value unchanged.
            Skip,
        }

        match Option::<PatchVal<T>>::deserialize(deserializer)? {
            //
            Some(PatchVal::Clear) => Ok(Self::Clear),

            Some(PatchVal::Assign(value)) => Ok(Self::Assign(value)),

            Some(PatchVal::Skip) | None => Ok(Self::Skip),
        }
    }
}
