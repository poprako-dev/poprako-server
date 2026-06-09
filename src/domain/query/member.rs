use async_trait::async_trait;

use poprako_macro::forward_ref;
use poprako_util::page::Page;

use crate::domain::model::aggr::member::{MemberAggr, MemberForm, MemberRoleUpdate};
use crate::domain::model::value::role::RoleFlag;
use crate::domain::result::DomainResult;

/// Persistence contract for [`MemberAggr`].
///
/// Each method takes an immutable `&self` reference, suitable for
/// non-transactional queries backed by a connection pool.
#[forward_ref]
#[async_trait]
pub trait MemberQuery {
    /// Returns the member with the given ID, or an expected error if not found.
    async fn get_by_id(&self, id: &str) -> DomainResult<MemberAggr>;

    /// Returns the member matching the given user and team IDs, or an expected error if not found.
    async fn get_by_user_and_team_id(
        &self,
        user_id: &str,
        team_id: &str,
    ) -> DomainResult<MemberAggr>;

    /// Lists members for the given team, with optional keyword and role filters.
    ///
    /// `keyword` performs an ILIKE search on `user_nickname`.
    /// `role` filters members that hold the given single role (IS NOT NULL on the column).
    async fn list(
        &self,
        team_id: &str,
        keyword: Option<&str>,
        role: Option<RoleFlag>,
        page: Page,
    ) -> DomainResult<Vec<MemberAggr>>;

    /// Returns whether a member exists for the given user and team IDs.
    async fn exist_by_user_and_team_id(&self, user_id: &str, team_id: &str) -> DomainResult<bool>;

    /// Updates the roles for a member (PUT-style: clears all role timestamps,
    /// then sets only those in the [`RoleMask`] to the current time).
    async fn update_roles(&self, update: &MemberRoleUpdate) -> DomainResult<()>;

    /// Hard-deletes the member with the given ID.
    async fn delete(&self, id: &str) -> DomainResult<()>;
}

/// Transactional persistence contract for [`MemberAggr`], used **only** inside
/// a transaction via [`QueryTransactional`](crate::domain::query::QueryTransactional).
#[async_trait]
pub trait MemberQueryTransactional {
    /// Inserts a new member row from the creation form.
    async fn create(&mut self, form: &MemberForm) -> DomainResult<MemberAggr>;

    /// Updates the nickname on all member rows belonging to the given user.
    async fn update_user_nickname(&mut self, user_id: &str, nickname: &str) -> DomainResult<()>;

    /// Updates the last active timestamp on all member rows belonging to the given user.
    async fn touch_last_active(&mut self, user_id: &str) -> DomainResult<()>;

    /// Lists all members belonging to the given user.
    async fn list_by_user_id_excluded(&mut self, user_id: &str) -> DomainResult<Vec<MemberAggr>>;

    /// Hard-deletes the member with the given ID.
    async fn delete_transactional(&mut self, id: &str) -> DomainResult<()>;
}
