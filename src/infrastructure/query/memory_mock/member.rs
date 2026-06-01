use async_trait::async_trait;
use time::OffsetDateTime;

use crate::domain::model::aggregate::member::{MemberAggr, MemberForm};
use crate::domain::model::value::role::RoleFlag;
use crate::domain::query::member::MemberQueryTransactional;
use crate::domain::result::{DomainError, DomainResult};
use crate::infrastructure::query::memory_mock::MemoryMockQueryTransactional;
use crate::util::err::ErrorTrace as _;
use crate::util::i18n::trl;

#[async_trait]
impl MemberQueryTransactional for MemoryMockQueryTransactional {
    async fn create(&mut self, form: &MemberForm) -> DomainResult<MemberAggr> {
        let mut state = self.state.lock().unwrap();

        // Check uniqueness constraints.
        if state.members.iter().any(|m| m.id == form.id) {
            return Err(DomainError::expected_conflict(trl("error-already-exists")))
                .trace_debug();
        }
        if state
            .members
            .iter()
            .any(|m| m.user_id == form.user_id && m.team_id == form.team_id)
        {
            return Err(DomainError::expected_conflict(trl("error-already-exists")))
                .trace_debug();
        }

        // Build the member aggregate from the form.
        let now = OffsetDateTime::now_utc();
        let roles = form.roles;

        let member = MemberAggr::new(
            form.id.clone(),
            form.user_id.clone(),
            None, // user not populated in mock
            form.team_id.clone(),
            None, // team not populated in mock
            roles.has_role(RoleFlag::RawProvider).then_some(now),
            roles.has_role(RoleFlag::Translator).then_some(now),
            roles.has_role(RoleFlag::Proofreader).then_some(now),
            roles.has_role(RoleFlag::Typesetter).then_some(now),
            roles.has_role(RoleFlag::Redrawer).then_some(now),
            roles.has_role(RoleFlag::Reviewer).then_some(now),
            roles.has_role(RoleFlag::Publisher).then_some(now),
            roles.has_role(RoleFlag::Admin).then_some(now),
            roles.has_role(RoleFlag::Assistant).then_some(now),
            now, // created_at
            now, // updated_at
        );

        state.members.push(member.clone());

        Ok(member)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::aggregate::member::MemberForm;
    use crate::domain::model::value::role::{RoleFlag, RoleMask};
    use crate::domain::query::Transactional;
    use crate::domain::result::{DomainError, ExpectedVariant};
    use crate::infrastructure::query::memory_mock::MemoryMockQuery;

    fn is_expected_conflict(err: &DomainError) -> bool {
        matches!(
            err,
            DomainError::Expected {
                variant: ExpectedVariant::Conflict,
                ..
            }
        )
    }

    #[tokio::test]
    async fn duplicate_user_team_returns_conflict() {
        let mock = MemoryMockQuery::new();

        mock.transaction_scoped(|txn| {
            Box::pin(async move {
                let form = MemberForm::new(
                    "user-1".into(),
                    "nick".into(),
                    "team-1".into(),
                    RoleMask::from(RoleFlag::Admin),
                );
                MemberQueryTransactional::create(txn, &form).await.unwrap();
                Ok(())
            })
        })
        .await
        .unwrap();

        let err = mock
            .transaction_scoped(|txn| {
                Box::pin(async move {
                    let form = MemberForm::new(
                        "user-1".into(),
                        "nick".into(),
                        "team-1".into(),
                        RoleMask::from(RoleFlag::Translator),
                    );
                    MemberQueryTransactional::create(txn, &form).await
                })
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_conflict(&err));
    }
}
