use async_trait::async_trait;

use crate::domain::model::aggregate::team::TeamAggr;
use crate::domain::result::DomainResult;

/// Read-only persistence contract for [`TeamAggr`].
///
/// Each method takes an immutable `&self` reference, suitable for
/// non-transactional queries backed by a connection pool.
#[async_trait]
pub trait TeamQuery {
    /// Returns the team with the given ID, or an expected error if not found.
    async fn get_by_id(&self, id: String) -> DomainResult<TeamAggr>;
}
