use async_trait::async_trait;
use time::OffsetDateTime;

use crate::domain::model::aggregate::member::{MemberAggr, MemberForm};
use crate::domain::model::value::role::RoleFlag;
use crate::domain::query::member::MemberQueryTransactional;
use crate::domain::result::{DomainError, DomainResult};
use crate::infrastructure::query::memory_mock::MemoryMockQueryTransactional;
use crate::util::i18n::trl;

#[async_trait]
impl MemberQueryTransactional for MemoryMockQueryTransactional {
    async fn create(&mut self, form: &MemberForm) -> DomainResult<MemberAggr> {
        let mut state = self.state.lock().unwrap();

        // Check uniqueness constraints.
        if state.members.iter().any(|m| m.id == form.id) {
            return Err(DomainError::expected_conflict(trl("error-already-exists")).trace());
        }
        if state
            .members
            .iter()
            .any(|m| m.user_id == form.user_id && m.team_id == form.team_id)
        {
            return Err(DomainError::expected_conflict(trl("error-already-exists")).trace());
        }

        // Build the member aggregate from the form.
        let now = OffsetDateTime::now_utc();
        let roles = form.roles;

        let member = MemberAggr {
            id: form.id.clone(),
            user_id: form.user_id.clone(),
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
            created_at: now,
            updated_at: now,
        };

        state.members.push(member.clone());

        Ok(member)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // duplicate_user_team_returns_conflict(MemberQueryTransactional::create)(negative): duplicate user-team membership should return an expected conflict.

    use crate::domain::model::aggregate::member::{MemberAggr, MemberForm};
    use crate::domain::model::value::role::{RoleFlag, RoleMask};
    use crate::domain::query::Transactional;
    use crate::domain::query::member::MemberQueryTransactional;
    use crate::infrastructure::query::memory_mock::MemoryMockQuery;
    use crate::test_util::is_expected_conflict;

    #[tokio::test]
    async fn duplicate_user_team_returns_conflict() {
        let mock = MemoryMockQuery::new();

        mock.transaction_scoped(|txn| {
            Box::pin(async move {
                let form = MemberForm {
                    id: MemberAggr::generate_id(),
                    user_id: "user-1".into(),
                    user_nickname: "nick".into(),
                    team_id: "team-1".into(),
                    roles: RoleMask::from(RoleFlag::Admin),
                };
                MemberQueryTransactional::create(txn, &form).await.unwrap();
                Ok(())
            })
        })
        .await
        .unwrap();

        let err = mock
            .transaction_scoped(|txn| {
                Box::pin(async move {
                    let form = MemberForm {
                        id: MemberAggr::generate_id(),
                        user_id: "user-1".into(),
                        user_nickname: "nick".into(),
                        team_id: "team-1".into(),
                        roles: RoleMask::from(RoleFlag::Translator),
                    };
                    MemberQueryTransactional::create(txn, &form).await
                })
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_conflict(&err));
    }
}
