use async_trait::async_trait;
use time::OffsetDateTime;

use poprako_util::i18n::trl;

use crate::domain::model::aggr::member::{MemberAggr, MemberForm};
use crate::domain::model::value::role::RoleFlag;
use crate::domain::query::member::MemberQueryTransactional;
use crate::domain::result::{DomainError, DomainResult};
use crate::infra::query::memory_mock::MemoryMockQueryTransactional;

#[async_trait]
impl MemberQueryTransactional for MemoryMockQueryTransactional {
    async fn create(&mut self, form: &MemberForm) -> DomainResult<MemberAggr> {
        let mut state = self.state.lock().unwrap();

        // Check uniqueness constraints.
        if state.members.iter().any(|m| m.id == form.id) {
            return Err(DomainError::expected_conflict(trl("error-already-exists")));
        }
        if state
            .members
            .iter()
            .any(|m| m.user_id == form.user_id && m.team_id == form.team_id)
        {
            return Err(DomainError::expected_conflict(trl("error-already-exists")));
        }

        // Build the member aggregate from the form.
        let now = OffsetDateTime::now_utc();
        let roles = form.roles;

        let member = MemberAggr {
            id: form.id.clone(),
            user_id: form.user_id.clone(),
            user_nickname: form.user_nickname.clone(),
            user: None, // user not populated in mock
            team_id: form.team_id.clone(),
            team: None, // team not populated in mock
            assigned_raw_provider_at: roles.has_role(RoleFlag::RawProvider).then_some(now),
            assigned_translator_at: roles.has_role(RoleFlag::Translator).then_some(now),
            assigned_proofreader_at: roles.has_role(RoleFlag::Proofreader).then_some(now),
            assigned_typesetter_at: roles.has_role(RoleFlag::Typesetter).then_some(now),
            assigned_redrawer_at: roles.has_role(RoleFlag::Redrawer).then_some(now),
            assigned_reviewer_at: roles.has_role(RoleFlag::Reviewer).then_some(now),
            assigned_publisher_at: roles.has_role(RoleFlag::Publisher).then_some(now),
            assigned_admin_at: roles.has_role(RoleFlag::Admin).then_some(now),
            assigned_assistant_at: roles.has_role(RoleFlag::Assistant).then_some(now),
            user_last_active_at: now,
            created_at: now,
            updated_at: now,
        };

        state.members.push(member.clone());

        Ok(member)
    }

    async fn update_user_nickname(&mut self, user_id: &str, nickname: &str) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();

        for member in state.members.iter_mut() {
            if member.user_id == user_id {
                member.user_nickname = nickname.to_string();
            }
        }

        Ok(())
    }

    async fn touch_last_active(&mut self, user_id: &str) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();
        let now = OffsetDateTime::now_utc();

        for member in state.members.iter_mut() {
            if member.user_id == user_id {
                member.user_last_active_at = now;
            }
        }

        Ok(())
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // duplicate_user_team_returns_conflict(MemberQueryTransactional::create)(negative): duplicate user-team membership should return an expected conflict.
    // update_user_nickname_updates_all_members_for_a_user(update_user_nickname)(positive): update_user_nickname should update the nickname on all members belonging to the user.
    // update_user_nickname_succeeds_when_user_has_no_members(update_user_nickname)(positive): update_user_nickname should succeed (no-op) when the user has no members.

    use futures_util::FutureExt as _;

    use crate::domain::model::aggr::member::{MemberAggr, MemberForm};
    use crate::domain::model::value::role::{RoleFlag, RoleMask};
    use crate::domain::query::Transactional;
    use crate::domain::query::member::MemberQueryTransactional;
    use crate::infra::query::memory_mock::MemoryMockQuery;
    use crate::test_util::is_expected_conflict;

    #[tokio::test]
    async fn duplicate_user_team_returns_conflict() {
        let mock = MemoryMockQuery::new();

        mock.transaction_scoped(|txn| {
            async move {
                let form = MemberForm {
                    id: MemberAggr::generate_id(),
                    user_id: "user-1".into(),
                    user_nickname: "nick".into(),
                    team_id: "team-1".into(),
                    roles: RoleMask::from(RoleFlag::Admin),
                };
                MemberQueryTransactional::create(txn, &form).await.unwrap();
                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    let form = MemberForm {
                        id: MemberAggr::generate_id(),
                        user_id: "user-1".into(),
                        user_nickname: "nick".into(),
                        team_id: "team-1".into(),
                        roles: RoleMask::from(RoleFlag::Translator),
                    };
                    MemberQueryTransactional::create(txn, &form).await
                }
                .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_conflict(&err));
    }

    #[tokio::test]
    async fn update_user_nickname_updates_all_members_for_a_user() {
        let mock = MemoryMockQuery::new();

        // Create two members for the same user in different teams.
        mock.transaction_scoped(|txn| {
            async move {
                let form1 = MemberForm {
                    id: MemberAggr::generate_id(),
                    user_id: "user-1".into(),
                    user_nickname: "OldNick".into(),
                    team_id: "team-1".into(),
                    roles: RoleMask::from(RoleFlag::Admin),
                };
                MemberQueryTransactional::create(txn, &form1).await.unwrap();

                let form2 = MemberForm {
                    id: MemberAggr::generate_id(),
                    user_id: "user-1".into(),
                    user_nickname: "OldNick".into(),
                    team_id: "team-2".into(),
                    roles: RoleMask::from(RoleFlag::Translator),
                };
                MemberQueryTransactional::create(txn, &form2).await.unwrap();

                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();

        // Update the nickname.
        mock.transaction_scoped(|txn| {
            async move {
                MemberQueryTransactional::update_user_nickname(
                    txn,
                    "user-1",
                    "NewNick",
                )
                .await
            }
            .boxed()
        })
        .await
        .unwrap();

        let snapshot = mock.snapshot();
        assert_eq!(snapshot.members.len(), 2);

        for member in &snapshot.members {
            assert_eq!(member.user_id, "user-1");
            assert_eq!(member.user_nickname, "NewNick");
        }
    }

    #[tokio::test]
    async fn update_user_nickname_succeeds_when_user_has_no_members() {
        let mock = MemoryMockQuery::new();

        let result = mock
            .transaction_scoped(|txn| {
                async move {
                    MemberQueryTransactional::update_user_nickname(txn, "no-such-user", "Nick")
                        .await
                }
                .boxed()
            })
            .await;

        assert!(result.is_ok());
    }
}
