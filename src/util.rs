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
    async fn transactional(&self) -> Self::Transactional;
}

/// Generate a unique time-ordered identifier in base64url format.
///
/// A u64 fits in 11 unpadded base64url characters (vs 16 hex chars).
pub fn next_snowflake_id() -> String {
    u64_to_base64url(next_snowflake_u64())
}

/// Encode a `u64` as an 11-character unpadded base64url string.
///
/// Base64url uses `-` and `_` instead of `+` and `/`, and omits padding —
/// the result is safe in URLs, JSON, and path segments without escaping.
fn u64_to_base64url(v: u64) -> String {
    const CHARS: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    let b = v.to_be_bytes();

    let mut buf = [0u8; 11];
    buf[0] = CHARS[(b[0] >> 2) as usize];
    buf[1] = CHARS[((b[0] & 0x03) << 4 | b[1] >> 4) as usize];
    buf[2] = CHARS[((b[1] & 0x0f) << 2 | b[2] >> 6) as usize];
    buf[3] = CHARS[(b[2] & 0x3f) as usize];
    buf[4] = CHARS[(b[3] >> 2) as usize];
    buf[5] = CHARS[((b[3] & 0x03) << 4 | b[4] >> 4) as usize];
    buf[6] = CHARS[((b[4] & 0x0f) << 2 | b[5] >> 6) as usize];
    buf[7] = CHARS[(b[5] & 0x3f) as usize];
    buf[8] = CHARS[(b[6] >> 2) as usize];
    buf[9] = CHARS[((b[6] & 0x03) << 4 | b[7] >> 4) as usize];
    buf[10] = CHARS[((b[7] & 0x0f) << 2) as usize];

    // SAFETY: buf only contains chars A-Z, a-z, 0-9, '-', '_' — all valid UTF-8.
    unsafe { String::from_utf8_unchecked(buf.to_vec()) }
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

    match value.parse::<u16>() {
        Ok(instance) if instance <= 1023 => instance,
        _ => unreachable!(),
    }
}

// pub trait Validate {
//     fn validate(&self) -> RootResult<()>;
// }

#[cfg(test)]
mod tests;
