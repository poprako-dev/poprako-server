use async_trait::async_trait;

use crate::domain::model::aggregate::team::TeamAggr;
use crate::domain::result::DomainResult;
use crate::util::ForwardRef;

/// Forwarding marker for [`TeamQuery`].
pub struct TeamQueryForward;

/// Persistence contract for [`TeamAggr`].
#[async_trait]
pub trait TeamQuery {
    /// Returns the team with the given ID, or an expected error if not found.
    async fn get_by_id(&self, id: &str) -> DomainResult<TeamAggr>;
}

#[async_trait]
impl<T> TeamQuery for T
where
    T: ForwardRef<TeamQueryForward> + Sync,
    T::Target: TeamQuery + Sync,
{
    async fn get_by_id(&self, id: &str) -> DomainResult<TeamAggr> {
        self.forward_ref().get_by_id(id).await
    }
}
