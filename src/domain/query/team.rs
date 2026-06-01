use async_trait::async_trait;

use crate::domain::model::aggregate::team::TeamAggr;
use crate::domain::result::DomainResult;

/// Persistence contract for [`TeamAggr`].
#[async_trait]
pub trait TeamQuery {
    /// Returns the team with the given ID, or an expected error if not found.
    async fn get_by_id(&self, id: &str) -> DomainResult<TeamAggr>;
}
