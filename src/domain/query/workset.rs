use async_trait::async_trait;

use poprako_macro::forward_ref;

use crate::domain::model::aggr::workset::{WorksetAggr, WorksetForm, WorksetUpdate};
use crate::domain::result::DomainResult;

/// Read-only persistence contract for [`WorksetAggr`].
///
/// Each method takes an immutable `&self` reference, suitable for
/// non-transactional queries backed by a connection pool.
#[forward_ref]
#[async_trait]
pub trait WorksetQuery {
    /// Returns the workset with the given ID, or an expected error if not found.
    async fn get_by_id(&self, id: &str) -> DomainResult<WorksetAggr>;

    /// Lists worksets for the given team, ordered by `index` ascending.
    ///
    /// The `offset` and `limit` parameters control pagination.
    /// Preloads the owning team on each workset.
    async fn list(&self, team_id: &str, offset: i64, limit: i64) -> DomainResult<Vec<WorksetAggr>>;

    /// Returns the total count of worksets for the given team.
    async fn count(&self, team_id: &str) -> DomainResult<i64>;
}

/// Mutable persistence contract for [`WorksetAggr`], used **only** inside
/// a transaction via [`QueryTransactional`](crate::domain::query::QueryTransactional).
#[async_trait]
pub trait WorksetQueryTransactional {
    /// Creates a new workset from the creation form inside a transaction.
    async fn create(&mut self, form: &WorksetForm) -> DomainResult<WorksetAggr>;

    /// Updates a workset's mutable fields (name, description) via PUT semantics.
    ///
    /// The `id` in the update input identifies the workset to modify.
    async fn update(&mut self, input: &WorksetUpdate) -> DomainResult<()>;

    /// Applies a delta to the `comic_count` counter of the workset.
    ///
    /// The counter is clamped to zero; it will never become negative.
    async fn update_comic_count(&mut self, id: &str, delta: i32) -> DomainResult<()>;

    /// Atomically increments and returns the next comic index from the workset-scoped sequence.
    ///
    /// Uses `UPDATE ... RETURNING` for atomic allocation.
    async fn increment_comic_next_index(&mut self, id: &str) -> DomainResult<i32>;

    /// Hard-deletes the workset with the given ID.
    async fn delete(&mut self, id: &str) -> DomainResult<()>;
}
