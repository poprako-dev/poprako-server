use async_trait::async_trait;
use time::OffsetDateTime;

use crate::domain::model::aggregate::user::{UserAggr, UserCredential, UserForm};
use crate::domain::query::user::{UserQuery, UserQueryTransactional};
use crate::domain::result::{DomainError, DomainResult};
use crate::infrastructure::query::memory_mock::{MemoryMockQuery, MemoryMockQueryTransactional};
use crate::util::err::ErrorTrace as _;
use crate::util::i18n::trl;

// ── Query impls ────────────────────────────────────────────────────────────

#[async_trait]
impl UserQuery for MemoryMockQuery {
    async fn get_by_id(&self, id: &str) -> DomainResult<UserAggr> {
        let state = self.state.lock().unwrap();
        state
            .users
            .iter()
            .find(|u| u.id == id)
            .cloned()
            .ok_or(DomainError::expected_argument(trl("error-user-not-found")))
            .trace_debug()
    }

    async fn get_credentials_by_qid(&self, qid: &str) -> DomainResult<UserCredential> {
        let state = self.state.lock().unwrap();
        state
            .credentials
            .iter()
            .find(|c| c.qid == qid)
            .cloned()
            .ok_or(DomainError::expected_argument(trl("error-user-not-found")))
            .trace_debug()
    }
}

// ── QueryTransactional impls ───────────────────────────────────────────────

#[async_trait]
impl UserQueryTransactional for MemoryMockQueryTransactional {
    async fn create(&mut self, form: &UserForm) -> DomainResult<UserAggr> {
        let mut state = self.state.lock().unwrap();

        // Check uniqueness constraints.
        if state.users.iter().any(|u| u.id == form.id) {
            return Err(DomainError::expected_conflict(trl("error-already-exists"))).trace_debug();
        }
        if state.users.iter().any(|u| u.qid == form.qid) {
            return Err(DomainError::expected_conflict(trl("error-already-exists"))).trace_debug();
        }
        if state.users.iter().any(|u| u.nickname == form.nickname) {
            return Err(DomainError::expected_conflict(trl("error-already-exists"))).trace_debug();
        }

        // Build the user aggregate from the form.
        let now = OffsetDateTime::now_utc();
        let user = UserAggr::new(
            form.id.clone(),
            form.nickname.clone(),
            form.qid.clone(),
            false,         // is_sadmin
            String::new(), // avatar_key
            false,         // avatar_uploaded
            now,           // last_active_at
            now,           // created_at
            now,           // updated_at
        );

        let credential = UserCredential::new(form.qid.clone(), form.password_hash.clone());

        state.users.push(user.clone());
        state.credentials.push(credential);

        Ok(user)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::aggregate::user::UserForm;
    use crate::domain::query::Transactional;
    use crate::domain::result::{DomainError, ExpectedVariant};
    use time::OffsetDateTime;

    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn make_user(id: &str, qid: &str, nickname: &str) -> UserAggr {
        UserAggr::new(
            id.into(),
            nickname.into(),
            qid.into(),
            false,
            String::new(),
            false,
            now(),
            now(),
            now(),
        )
    }

    fn make_credential(qid: &str) -> UserCredential {
        UserCredential::new(qid.into(), "hashed-pw".into())
    }

    fn is_expected_argument(err: &DomainError) -> bool {
        matches!(
            err,
            DomainError::Expected {
                variant: ExpectedVariant::Argument,
                ..
            }
        )
    }

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
    async fn find_by_id_after_seed() {
        let mock = MemoryMockQuery::new();
        mock.seed_user(
            make_user("user-1", "qid-1", "nick"),
            make_credential("qid-1"),
        );

        let found = UserQuery::get_by_id(&mock, "user-1").await.unwrap();
        assert_eq!(found.id, "user-1");
        assert_eq!(found.qid, "qid-1");
    }

    #[tokio::test]
    async fn find_credential_by_qid_after_seed() {
        let mock = MemoryMockQuery::new();
        mock.seed_user(
            make_user("user-1", "qid-1", "nick"),
            make_credential("qid-1"),
        );

        let cred = UserQuery::get_credentials_by_qid(&mock, "qid-1")
            .await
            .unwrap();
        assert_eq!(cred.qid, "qid-1");
        assert_eq!(cred.password_hash, "hashed-pw");
    }

    #[tokio::test]
    async fn get_by_id_missing_returns_expected_error() {
        let mock = MemoryMockQuery::new();

        let err = UserQuery::get_by_id(&mock, "nonexistent")
            .await
            .err()
            .unwrap();
        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn create_then_find() {
        let mock = MemoryMockQuery::new();

        mock.transaction_scoped(|txn| {
            Box::pin(async move {
                let form = UserForm::new("qid-new".into(), "nick-new".into(), "pw".into());
                let user = UserQueryTransactional::create(txn, &form).await.unwrap();
                assert_eq!(user.qid, "qid-new");
                assert_eq!(user.nickname, "nick-new");
                Ok(())
            })
        })
        .await
        .unwrap();

        // User readable from outside the transaction.
        let found = UserQuery::get_by_id(&mock, &mock.snapshot().users[0].id)
            .await
            .unwrap();
        assert_eq!(found.qid, "qid-new");
    }

    #[tokio::test]
    async fn duplicate_qid_returns_conflict() {
        let mock = MemoryMockQuery::new();

        mock.transaction_scoped(|txn| {
            Box::pin(async move {
                let form = UserForm::new("dup-qid".into(), "nick-1".into(), "pw".into());
                UserQueryTransactional::create(txn, &form).await.unwrap();
                Ok(())
            })
        })
        .await
        .unwrap();

        let err = mock
            .transaction_scoped(|txn| {
                Box::pin(async move {
                    let form = UserForm::new("dup-qid".into(), "nick-2".into(), "pw".into());
                    UserQueryTransactional::create(txn, &form).await
                })
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_conflict(&err));
    }

    #[tokio::test]
    async fn duplicate_nickname_returns_conflict() {
        let mock = MemoryMockQuery::new();

        mock.transaction_scoped(|txn| {
            Box::pin(async move {
                let form = UserForm::new("qid-1".into(), "dup-nick".into(), "pw".into());
                UserQueryTransactional::create(txn, &form).await.unwrap();
                Ok(())
            })
        })
        .await
        .unwrap();

        let err = mock
            .transaction_scoped(|txn| {
                Box::pin(async move {
                    let form = UserForm::new("qid-2".into(), "dup-nick".into(), "pw".into());
                    UserQueryTransactional::create(txn, &form).await
                })
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_conflict(&err));
    }
}
