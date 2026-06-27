//! Reusable traits and helpers shared across domain modules.

use std::sync::OnceLock;

use async_trait::async_trait;

/// Capability to produce a transactional drive clone from a non-transactional
/// reference. Implementations wrap a database connection pool and spawn a
/// new transaction for each call.
#[async_trait]
pub trait DeriveTransactional {
    // Transactional variant of Implementation type.
    type Transactional;

    async fn transactional(&self) -> Self::Transactional;
}

pub fn next_snowflake_id() -> String {
    format!("{:016x}", next_snowflake_u64())
}

pub fn next_snowflake_u64() -> u64 {
    // Make sure snowflake instance is initialized yet.
    ensure_snowflake_init();

    // Generate a snowflake id.
    k_snowflake::create_snowflake().to_decimal() as u64
}

fn ensure_snowflake_init() {
    // Only init snowflake instance once.
    static INIT_GURAD: OnceLock<()> = OnceLock::new();

    INIT_GURAD.get_or_init(|| k_snowflake::set_instance(load_snowflake_node_id()));
}

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
mod tests {
    // next_snowflake_u64(next_snowflake_u64)(positive): generated snowflake values should be monotonic and fit into u64.
    // next_snowflake_id(next_snowflake_id)(positive): generated snowflake strings should be hexadecimal and parse back to u64.

    use super::*;

    #[test]
    fn next_snowflake_u64_generates_monotonic_ids() {
        let first_id = next_snowflake_u64();

        let second_id = next_snowflake_u64();

        assert!(second_id > first_id);
    }

    #[test]
    fn next_snowflake_id_generates_hex_string() {
        let snowflake_id = next_snowflake_id();

        let parsed_snowflake_id = u64::from_str_radix(&snowflake_id, 16);

        assert!(parsed_snowflake_id.is_ok());
    }
}
