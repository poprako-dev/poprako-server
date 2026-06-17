use async_trait::async_trait;

use poprako_macro::forward_ref;
use poprako_util::page::Page;

use crate::domain::model::aggr::team::{TeamAggr, TeamAvatarReservation, TeamForm, TeamInfoUpdate};
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

    /// Creates a new team from the creation form.
    async fn create(&self, form: &TeamForm) -> DomainResult<TeamAggr>;

    /// Updates a team's mutable fields (name, description) via PUT semantics.
    async fn update_info(&self, update: &TeamInfoUpdate) -> DomainResult<()>;

    /// Marks the team's current avatar version as uploaded.
    async fn mark_avatar_uploaded(&self, id: &str, avatar_version: i64) -> DomainResult<()>;
}

/// Transactional persistence contract for [`TeamAggr`], used **only** inside
/// a transaction via [`QueryTransactional`](crate::domain::query_legacy::QueryTransactional).
#[async_trait]
pub trait TeamQueryTransactional {
    /// Atomically increments and returns the next workset index from the team-scoped sequence.
    async fn increment_workset_next_index(&mut self, id: &str) -> DomainResult<i32>;

    /// Returns the team inside a transaction, or an expected error if not found.
    async fn get_by_id_excluded(&mut self, id: &str) -> DomainResult<TeamAggr>;

    /// Reserves the next avatar object key and clears the uploaded flag.
    async fn reserve_avatar(
        &mut self,
        id: &str,
        file_extension: &str,
    ) -> DomainResult<TeamAvatarReservation>;

    /// Marks the team's current avatar version as uploaded.
    async fn mark_avatar_uploaded(&mut self, id: &str, avatar_version: i64) -> DomainResult<()>;

    /// Hard-deletes the team.
    async fn delete(&mut self, id: &str) -> DomainResult<()>;
}
