use crate::domain::model::aggregate::member_invitation::MemberInvitation;
use crate::domain::result::DomainResl;

#[async_trait::async_trait]
pub trait MemberInvitationQueryMut {
    async fn get_pending_by_invitee_qid(
        &mut self,
        invitee_qid: &str,
    ) -> DomainResl<MemberInvitation>;

    async fn mark_as_used(&mut self, id: &str) -> DomainResl<()>;
}
