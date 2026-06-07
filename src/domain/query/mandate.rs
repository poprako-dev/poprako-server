use async_trait::async_trait;

use poprako_macro::forward_ref;
use poprako_util::page::Page;

use crate::domain::model::aggr::mandate::{MandateAggr, MandateForm, MandateMark};
use crate::domain::result::DomainResult;

/// Persistence contract for consuming and managing local mandates.
///
/// Consumer-side state changes are guarded by the lease carried in each
/// [`MandateMark`]. If the stored lease no longer matches, the implementation
/// must fail the mark instead of retrying a stale worker result.
#[forward_ref]
#[async_trait]
pub trait MandateQuery {
    /// Claims a visible batch for the given topic and marks each returned mandate as processing.
    async fn claim(&self, topic: &str, limit: i64) -> DomainResult<Vec<MandateAggr>>;

    /// Applies consumer-side state changes guarded by each mark's lease.
    async fn mark(&self, marks: &[&MandateMark]) -> DomainResult<()>;

    /// Lists dead mandates for the given topic, ordered by `updated_at` descending.
    async fn list_dead(&self, topic: &str, page: Page) -> DomainResult<Vec<MandateAggr>>;

    /// Purges completed mandates for the given topic.
    async fn purge_completed(&self, topic: &str) -> DomainResult<()>;

    /// Hard-deletes dead mandates after manual investigation.
    async fn delete_dead(&self, items: &[&str]) -> DomainResult<()>;
}

/// Mutable persistence contract for appending local mandates inside a transaction.
#[async_trait]
pub trait MandateQueryTransactional {
    /// Appends a mandate and makes it visible only after the transaction commits.
    async fn append(&mut self, form: &MandateForm) -> DomainResult<MandateAggr>;
}
