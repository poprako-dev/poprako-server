//! Reusable traits and helpers shared across domain modules.

use async_trait::async_trait;

/// Capability to produce a transactional drive clone from a non-transactional
/// reference. Implementations wrap a database connection pool and spawn a
/// new transaction for each call.
#[async_trait]
pub trait DeriveTransactional {
    type Transactional;

    async fn transactional(&self) -> Self::Transactional;
}
