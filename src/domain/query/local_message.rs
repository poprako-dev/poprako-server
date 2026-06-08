use async_trait::async_trait;

use poprako_macro::forward_ref;
use poprako_util::page::Page;

use crate::domain::model::aggr::local_message::{
    LocalMessageAggr, LocalMessageForm, LocalMessageMark,
};
use crate::domain::result::DomainResult;

/// Persistence contract for consuming and managing local messages.
///
/// Consumer-side state changes are guarded by the lease carried in each
/// [`LocalMessageMark`]. If the stored lease no longer matches, the implementation
/// must fail the mark instead of retrying a stale worker result.
#[forward_ref]
#[async_trait]
pub trait LocalMessageQuery {
    /// Claims a visible batch for the given topic and marks each returned message as processing.
    async fn claim(&self, topic: &str, limit: i64) -> DomainResult<Vec<LocalMessageAggr>>;

    /// Applies consumer-side state changes guarded by each mark's lease.
    async fn mark(&self, marks: &[&LocalMessageMark]) -> DomainResult<()>;

    /// Lists dead messages for the given topic, ordered by `updated_at` descending.
    async fn list_dead(&self, topic: &str, page: Page) -> DomainResult<Vec<LocalMessageAggr>>;

    /// Purges completed messages for the given topic.
    async fn purge_completed(&self, topic: &str) -> DomainResult<()>;

    /// Hard-deletes dead messages after manual investigation.
    async fn delete_dead(&self, items: &[&str]) -> DomainResult<()>;
}

/// Mutable persistence contract for appending local messages inside a transaction.
#[async_trait]
pub trait LocalMessageQueryTransactional {
    /// Appends a message and makes it visible only after the transaction commits.
    async fn append(&mut self, form: &LocalMessageForm) -> DomainResult<LocalMessageAggr>;

    /// Applies consumer-side state changes inside the current transaction.
    async fn mark(&mut self, marks: &[&LocalMessageMark]) -> DomainResult<()>;
}
