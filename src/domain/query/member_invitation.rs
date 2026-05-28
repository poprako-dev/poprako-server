use crate::domain::model::aggregate::member_invitation::MemberInvitation;
use crate::domain::result::DomainResl;

/// Mutable persistence contract for [`MemberInvitation`](crate::domain::model::aggregate::member_invitation::MemberInvitation),
/// used **only** inside a transaction via [`TransactionalQuery`](crate::domain::query::TransactionalQuery).
#[async_trait::async_trait]
pub trait MemberInvitationQueryMut {
    /// Returns the most recent pending invitation for the given invitee qualified ID,
    /// or an expected error if none exists.
    async fn get_pending_by_invitee_qid(
        &mut self,
        invitee_qid: &str,
    ) -> DomainResl<MemberInvitation>;

    /// Marks an invitation as no longer pending (i.e. consumed), without deleting the row.
    async fn mark_as_used(&mut self, id: &str) -> DomainResl<()>;
}
