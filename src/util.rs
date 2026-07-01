//! Reusable traits and helpers shared across domain modules.

use std::sync::OnceLock;

use async_trait::async_trait;

/// Capability to produce a transactional drive clone from a non-transactional
/// reference. Implementations wrap a database connection pool and spawn a
/// new transaction for each call.
#[async_trait]
pub trait DeriveTransactional {
    /// Transactional variant of the implementation type.
    type Transactional;

    /// Obtain a transactional handle from a non-transactional reference.
    async fn derive_transactional(&self) -> Self::Transactional;
}

/// Generate a unique time-ordered identifier in base62 format.
///
/// A u64 fits in 11 base62 characters (vs 16 hex chars).
pub fn next_snowflake_id() -> String {
    base62::encode(next_snowflake_u64())
}

/// Generate a unique time-ordered 64-bit value backed by a snowflake.
pub fn next_snowflake_u64() -> u64 {
    // Make sure snowflake instance is initialized yet.
    ensure_snowflake_init();

    // Generate a snowflake id.
    k_snowflake::create_snowflake().to_decimal() as u64
}

/// Initialise the global snowflake instance once from the
/// `POPRAKO_SNOWFLAKE_NODE_ID` env var (defaults to 0).
fn ensure_snowflake_init() {
    // Only init snowflake instance once.
    static INIT_GURAD: OnceLock<()> = OnceLock::new();

    INIT_GURAD.get_or_init(|| k_snowflake::set_instance(load_snowflake_node_id()));
}

/// Load the snowflake node ID from the `POPRAKO_SNOWFLAKE_NODE_ID` env var.
fn load_snowflake_node_id() -> u16 {
    let value = match std::env::var("POPRAKO_SNOWFLAKE_NODE_ID") {
        Ok(value) => value,
        Err(_) => return 0,
    };

    let parsed: Result<u16, _> = value.parse();
    match parsed {
        Ok(instance) if instance <= 1023 => instance,
        _ => unreachable!(),
    }
}

// pub trait Validate {
//     fn validate(&self) -> RootResult<()>;
// }

#[cfg(test)]
mod tests;
