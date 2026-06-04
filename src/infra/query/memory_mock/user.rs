use async_trait::async_trait;
use time::OffsetDateTime;

use crate::domain::model::aggr::user::{UserAggr, UserCredential, UserForm};
use crate::domain::query::user::{UserQuery, UserQueryTransactional};
use crate::domain::result::{DomainError, DomainResult};
use crate::infra::query::memory_mock::{MemoryMockQuery, MemoryMockQueryTransactional};
use poprako_util::i18n::trl;

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
            .ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")).trace())
    }

    async fn get_credentials_by_qid(&self, qid: &str) -> DomainResult<UserCredential> {
        let state = self.state.lock().unwrap();
        state
            .credentials
            .iter()
            .find(|c| c.qid == qid)
            .cloned()
            .ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")).trace())
    }
}

// ── QueryTransactional impls ───────────────────────────────────────────────

#[async_trait]
impl UserQueryTransactional for MemoryMockQueryTransactional {
    async fn create(&mut self, form: &UserForm) -> DomainResult<UserAggr> {
        let mut state = self.state.lock().unwrap();

        // Check uniqueness constraints.
        if state.users.iter().any(|u| u.id == form.id) {
            return Err(DomainError::expected_conflict(trl("error-already-exists")).trace());
        }
        if state.users.iter().any(|u| u.qid == form.qid) {
            return Err(DomainError::expected_conflict(trl("error-already-exists")).trace());
        }
        if state.users.iter().any(|u| u.nickname == form.nickname) {
            return Err(DomainError::expected_conflict(trl("error-already-exists")).trace());
        }

        // Build the user aggregate from the form.
        let now = OffsetDateTime::now_utc();
        let user = UserAggr {
            id: form.id.clone(),
            nickname: form.nickname.clone(),
            qid: form.qid.clone(),
            is_sadmin: false,
            avatar_key: String::new(),
            avatar_uploaded: false,
            last_active_at: now,
            created_at: now,
            updated_at: now,
        };

        let credential = UserCredential {
            qid: form.qid.clone(),
            password_hash: form.password_hash.clone(),
        };

        state.users.push(user.clone());
        state.credentials.push(credential);

        Ok(user)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // find_by_id_after_seed(UserQuery::get_by_id)(positive): seeded users should be found by ID.
    // find_credential_by_qid_after_seed(UserQuery::get_credentials_by_qid)(positive): seeded credentials should be found by qualified ID.
    // get_by_id_missing_returns_expected_error(UserQuery::get_by_id)(negative): missing users should return an expected argument error.
    // create_then_find(UserQueryTransactional::create)(positive): created users should be readable after transaction commit.
    // duplicate_qid_returns_conflict(UserQueryTransactional::create)(negative): duplicate qualified IDs should return an expected conflict.
    // duplicate_nickname_returns_conflict(UserQueryTransactional::create)(negative): duplicate nicknames should return an expected conflict.

    use time::OffsetDateTime;

    use crate::domain::model::aggr::user::{UserAggr, UserCredential, UserForm};
    use crate::domain::query::Transactional;
    use crate::domain::query::user::UserQuery;
    use crate::domain::query::user::UserQueryTransactional;
    use crate::infra::query::memory_mock::MemoryMockQuery;
    use crate::test_util::is_expected_argument;
    use crate::test_util::is_expected_conflict;

    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn make_user(id: &str, qid: &str, nickname: &str) -> UserAggr {
        let n = now();
        UserAggr {
            id: id.into(),
            nickname: nickname.into(),
            qid: qid.into(),
            is_sadmin: false,
            avatar_key: String::new(),
            avatar_uploaded: false,
            last_active_at: n,
            created_at: n,
            updated_at: n,
        }
    }

    fn make_credential(qid: &str) -> UserCredential {
        UserCredential {
            qid: qid.into(),
            password_hash: "hashed-pw".into(),
        }
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
                let form = UserForm::new(
                    UserAggr::generate_id(),
                    "qid-new".into(),
                    "nick-new".into(),
                    "pw".into(),
                );
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
                let form = UserForm::new(
                    UserAggr::generate_id(),
                    "dup-qid".into(),
                    "nick-1".into(),
                    "pw".into(),
                );
                UserQueryTransactional::create(txn, &form).await.unwrap();
                Ok(())
            })
        })
        .await
        .unwrap();

        let err = mock
            .transaction_scoped(|txn| {
                Box::pin(async move {
                    let form = UserForm::new(
                        UserAggr::generate_id(),
                        "dup-qid".into(),
                        "nick-2".into(),
                        "pw".into(),
                    );
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
                let form = UserForm::new(
                    UserAggr::generate_id(),
                    "qid-1".into(),
                    "dup-nick".into(),
                    "pw".into(),
                );
                UserQueryTransactional::create(txn, &form).await.unwrap();
                Ok(())
            })
        })
        .await
        .unwrap();

        let err = mock
            .transaction_scoped(|txn| {
                Box::pin(async move {
                    let form = UserForm::new(
                        UserAggr::generate_id(),
                        "qid-2".into(),
                        "dup-nick".into(),
                        "pw".into(),
                    );
                    UserQueryTransactional::create(txn, &form).await
                })
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_conflict(&err));
    }
}
