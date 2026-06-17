use async_trait::async_trait;

use crate::domain::model::aggr::member_invitation::MemberInvitationAggr;
use crate::domain::result::DomainResult;

/// Mutable persistence contract for [`MemberInvitationAggr`], used **only**
/// inside a transaction via [`QueryTransactional`](crate::domain::query_legacy::QueryTransactional).
#[async_trait]
pub trait MemberInvitationQueryTransactional {
    /// Returns the pending invitation for the given invitation code,
    /// acquiring an exclusive row-level lock (`SELECT ... FOR UPDATE`).
    ///
    /// The lock is held until the enclosing transaction commits or rolls back,
    /// preventing concurrent consumption of the same invitation.
    async fn get_by_code_excluded(
        &mut self,
        invitation_code: &str,
    ) -> DomainResult<MemberInvitationAggr>;

    /// Marks an invitation as consumed by atomically clearing `f_pending`.
    ///
    /// The update is conditional on `f_pending = true` so it is safe to call
    /// even when the row lock acquired by [`get_by_code_ex`](Self::get_by_code_ex)
    /// has already been released.
    async fn mark_pending_as_used(&mut self, id: &str) -> DomainResult<()>;
}
