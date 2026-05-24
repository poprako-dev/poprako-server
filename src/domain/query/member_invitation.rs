use crate::domain::model::aggr::member_invitation::MemberInvitation;
use crate::domain::query::QueryRetVal;

#[async_trait::async_trait]
pub trait MemberInvitationQueryMut {
    async fn get_pending_by_invitee_qid(
        &mut self,
        invitee_qid: &str,
    ) -> QueryRetVal<MemberInvitation>;

    async fn mark_as_used(&mut self, id: &str) -> QueryRetVal<()>;
}
