use async_trait::async_trait;

use poprako_macro::forward_ref;
use poprako_util::page::Page;

use crate::domain::model::aggr::team::{TeamAggr, TeamForm, TeamUpdate};
use crate::domain::result::DomainResult;

/// Persistence contract for [`TeamAggr`].
///
/// Each method takes an immutable `&self` reference, suitable for
/// non-transactional queries backed by a connection pool.
#[forward_ref]
#[async_trait]
pub trait TeamQuery {
    /// Returns the team with the given ID, or an expected error if not found.
    async fn get_by_id(&self, id: &str) -> DomainResult<TeamAggr>;

    /// Lists all teams ordered by `created_at` descending.
    ///
    /// The `page` parameter controls pagination.
    async fn list(&self, page: Page) -> DomainResult<Vec<TeamAggr>>;

    /// Reserves an avatar key for the team before the client uploads the file.
    ///
    /// Sets `avatar_key` to the given key, `avatar_uploaded` to `false`,
    /// and refreshes `updated_at`.
    async fn prefill_avatar_key(&self, id: &str, key: &str) -> DomainResult<()>;

    /// Marks the team's avatar as successfully uploaded.
    async fn mark_avatar_uploaded(&self, id: &str) -> DomainResult<()>;

    /// Creates a new team from the creation form.
    async fn create(&self, form: &TeamForm) -> DomainResult<TeamAggr>;

    /// Updates a team's mutable fields (name, description) via PUT semantics.
    async fn update(&self, input: &TeamUpdate) -> DomainResult<()>;

    /// Hard-deletes the team with the given ID.
    async fn delete(&self, id: &str) -> DomainResult<()>;
}

/// Transactional persistence contract for [`TeamAggr`], used **only** inside
/// a transaction via [`QueryTransactional`](crate::domain::query::QueryTransactional).
#[async_trait]
pub trait TeamQueryTransactional {
    /// Atomically increments and returns the next workset index from the team-scoped sequence.
    async fn increment_workset_next_index(&mut self, id: &str) -> DomainResult<i32>;
}
