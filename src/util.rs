//! Reusable traits and helpers shared across domain modules.

use async_trait::async_trait;

use crate::result::RootResult;

/// Capability to produce a transactional drive clone from a non-transactional
/// reference. Implementations wrap a database connection pool and spawn a
/// new transaction for each call.
#[async_trait]
pub trait DeriveTransactional {
    // Transactional variant of Implementation type.
    type Transactional;

    async fn transactional(&self) -> Self::Transactional;
}

// pub trait Validate {
//     fn validate(&self) -> RootResult<()>;
// }
