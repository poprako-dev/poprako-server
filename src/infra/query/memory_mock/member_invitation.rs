use async_trait::async_trait;

use crate::domain::model::aggr::member_invitation::MemberInvitationAggr;
use crate::domain::query::member_invitation::MemberInvitationQueryTransactional;
use crate::domain::result::{DomainError, DomainResult};
use crate::infra::query::memory_mock::MemoryMockQueryTransactional;
use poprako_util::i18n::trl;

#[async_trait]
impl MemberInvitationQueryTransactional for MemoryMockQueryTransactional {
    async fn get_by_code_ex(
        &mut self,
        invitation_code: &str,
    ) -> DomainResult<MemberInvitationAggr> {
        let state = self.state.lock().unwrap();
        state
            .member_invitations
            .iter()
            .find(|inv| inv.code == invitation_code && inv.pending)
            .cloned()
            .ok_or_else(|| DomainError::expected_argument(trl("error-no-pending-invitation")))
    }

    async fn mark_pending_as_used(&mut self, id: &str) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();

        let idx = state
            .member_invitations
            .iter()
            .position(|inv| inv.id == id && inv.pending);

        match idx {
            Some(pos) => {
                state.member_invitations[pos].pending = false;
                Ok(())
            }
            None => Err(DomainError::expected_argument(trl(
                "error-invitation-not-found",
            ))),
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // find_pending_by_code(MemberInvitationQueryTransactional::get_by_code_ex)(positive): pending invitations should be found by code.
    // get_by_code_ex_no_pending_returns_expected_error(MemberInvitationQueryTransactional::get_by_code_ex)(negative): missing pending invitations should return an expected argument error.
    // mark_pending_as_used_then_not_found(MemberInvitationQueryTransactional::mark_pending_as_used)(positive): marking an invitation used should make it unavailable by code.
    // mark_pending_as_used_nonexistent_returns_error(MemberInvitationQueryTransactional::mark_pending_as_used)(negative): marking an unknown invitation should return an expected argument error.

    use futures_util::FutureExt as _;
    use time::OffsetDateTime;

    use crate::domain::model::aggr::member_invitation::MemberInvitationAggr;
    use crate::domain::model::value::role::{RoleFlag, RoleMask};
    use crate::domain::query::Transactional;
    use crate::domain::query::member_invitation::MemberInvitationQueryTransactional;
    use crate::infra::query::memory_mock::MemoryMockQuery;
    use crate::test_util::is_expected_argument;

    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn make_invitation(id: &str, code: &str, pending: bool) -> MemberInvitationAggr {
        MemberInvitationAggr {
            id: id.into(),
            invitor_id: "invitor-1".into(),
            invitor: None,
            team_id: "team-1".into(),
            invitee_qid: "invitee-qid".into(),
            code: code.into(),
            pending,
            roles: RoleMask::from(RoleFlag::Admin),
            created_at: now(),
        }
    }

    #[tokio::test]
    async fn find_pending_by_code() {
        let mock = MemoryMockQuery::new();
        mock.seed_member_invitation(make_invitation("inv-1", "CODE123", true));

        mock.transaction_scoped(|txn| {
            async move {
                let inv = MemberInvitationQueryTransactional::get_by_code_ex(txn, "CODE123")
                    .await
                    .unwrap();
                assert_eq!(inv.id, "inv-1");
                assert!(inv.pending);
                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn get_by_code_ex_no_pending_returns_expected_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move { MemberInvitationQueryTransactional::get_by_code_ex(txn, "NOPE").await }
                    .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn mark_pending_as_used_then_not_found() {
        let mock = MemoryMockQuery::new();
        mock.seed_member_invitation(make_invitation("inv-1", "CODE123", true));

        mock.transaction_scoped(|txn| {
            async move {
                MemberInvitationQueryTransactional::mark_pending_as_used(txn, "inv-1")
                    .await
                    .unwrap();
                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    MemberInvitationQueryTransactional::get_by_code_ex(txn, "CODE123").await
                }
                .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn mark_pending_as_used_nonexistent_returns_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    MemberInvitationQueryTransactional::mark_pending_as_used(txn, "nope").await
                }
                .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }
}
