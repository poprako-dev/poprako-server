//! Reusable traits and helpers shared across domain modules.

use std::sync::OnceLock;

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

pub enum PatchField<T> {
    Clear,
    Assign(T),
    Skip,
}

impl<T> PatchField<T> {
    pub const fn is_skip(&self) -> bool {
        matches!(self, Self::Skip)
    }
}
