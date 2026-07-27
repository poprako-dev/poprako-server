//! Reusable traits and helpers shared across domain modules.

use std::sync::OnceLock;

use serde::{Deserialize, Deserializer, Serialize};

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

// pub fn to_roman_style(mut num: u8) -> String {
//     const TABLE: &[(u8, &str)] = &[
//         (100, "c"),
//         (90, "xc"),
//         (50, "l"),
//         (40, "xl"),
//         (10, "x"),
//         (9, "ix"),
//         (5, "v"),
//         (4, "iv"),
//         (1, "i"),
//     ];
//
//     let mut ret = String::new();
//
//     for &(value, symbol) in TABLE {
//         while num >= value {
//             ret.push_str(symbol);
//             num -= value;
//         }
//     }
//
//     ret
// }

/// Initialise the global snowflake instance once from the
/// `POPRAKO_SNOWFLAKE_NODE_ID` env var (defaults to 0).
/// Initialise the global snowflake instance once from the
/// `POPRAKO_SNOWFLAKE_NODE_ID` env var (defaults to 0).
fn ensure_snowflake_init() {
    //
    // Only init snowflake instance once.
    static INIT_GURAD: OnceLock<()> = OnceLock::new();

    INIT_GURAD
        .get_or_init(|| k_snowflake::set_instance(load_snowflake_node_id()));
}

/// Load the snowflake node ID from the `POPRAKO_SNOWFLAKE_NODE_ID` env var.
/// Load the snowflake node ID from the `POPRAKO_SNOWFLAKE_NODE_ID` env var.
fn load_snowflake_node_id() -> u16 {
    //
    let value = match std::env::var("POPRAKO_SNOWFLAKE_NODE_ID") {
        //
        Ok(value) => value,

        Err(_) => return 0,
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
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Patch<T> {
    /// Resets the stored field.
    Clear,

    /// Replaces the stored field with the carried value.
    Assign(T),

    /// Preserves the stored field.
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
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Option::<T>::deserialize(deserializer)? {
            //
            Some(value) => Ok(Self::Assign(value)),

            None => Ok(Self::Clear),
        }
    }
}

impl<T> Default for Patch<T> {
    fn default() -> Self {
        Self::Skip
    }
}
