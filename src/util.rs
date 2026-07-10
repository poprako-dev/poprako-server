//! Reusable traits and helpers shared across domain modules.

use std::sync::OnceLock;

use async_trait::async_trait;
use bitcode::{Decode, Encode};

use crate::result::{RegularError, RegularResult};

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

/// Encode an archive payload with bitcode and compress it with Zstd.
///
/// SAFETY: Any encoding-structure or bitcode-version change must be preceded
/// by a full rewrite using a version that can read all existing archive data.
/// Bitcode does not promise stable formats across major versions.
pub fn compress_archive<T>(archive_payload: &T) -> RegularResult<Vec<u8>>
where
    T: Encode + ?Sized,
{
    let encoded_bytes = bitcode::encode(archive_payload);

    zstd::stream::encode_all(encoded_bytes.as_slice(), 0).map_err(|error| {
        RegularError::Unrecoverable {
            message: format!(
                "[util::compress_archive] failed to compress archive payload: {}",
                error
            ),
        }
    })
}

/// Decompress an archive payload and decode it with the pinned bitcode format.
// TODO: list read.
pub fn decompress_archive<T>(archived_bytes: &[u8]) -> RegularResult<T>
where
    T: for<'a> Decode<'a>,
{
    let encoded_bytes = zstd::stream::decode_all(archived_bytes).map_err(|error| {
        RegularError::Unrecoverable {
            message: format!(
                "[util::decompress_archive] failed to decompress archive payload: {}",
                error
            ),
        }
    })?;

    bitcode::decode(&encoded_bytes).map_err(|error| RegularError::Unrecoverable {
        message: format!(
            "[util::decompress_archive] failed to decode archive payload: {}",
            error
        ),
    })
}

/// Initialise the global snowflake instance once from the
/// `POPRAKO_SNOWFLAKE_NODE_ID` env var (defaults to 0).
/// Initialise the global snowflake instance once from the
/// `POPRAKO_SNOWFLAKE_NODE_ID` env var (defaults to 0).
fn ensure_snowflake_init() {
    // Only init snowflake instance once.
    static INIT_GURAD: OnceLock<()> = OnceLock::new();

    INIT_GURAD
        .get_or_init(|| k_snowflake::set_instance(load_snowflake_node_id()));
}

/// Load the snowflake node ID from the `POPRAKO_SNOWFLAKE_NODE_ID` env var.
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

#[cfg(test)]
mod tests;
