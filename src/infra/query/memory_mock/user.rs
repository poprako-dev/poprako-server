use async_trait::async_trait;
use time::OffsetDateTime;

use poprako_util::i18n::trl;

use crate::domain::model::aggr::user::{
    UserAggr, UserAvatarReservation, UserCredential, UserForm, UserInfoUpdate,
};
use crate::domain::query::user::{UserQuery, UserQueryTransactional};
use crate::domain::result::{DomainError, DomainResult};
use crate::infra::query::memory_mock::{MemoryMockQuery, MemoryMockQueryTransactional};

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
            .ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")))
    }

    async fn get_credentials_by_qid(&self, qid: &str) -> DomainResult<UserCredential> {
        let state = self.state.lock().unwrap();
        let user = state
            .users
            .iter()
            .find(|u| u.qid == qid)
            .ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")))?;
        state
            .credentials
            .iter()
            .find(|c| c.user_id == user.id)
            .cloned()
            .ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")))
    }
}

// ── QueryTransactional impls ───────────────────────────────────────────────

#[async_trait]
impl UserQueryTransactional for MemoryMockQueryTransactional {
    async fn create(&mut self, form: &UserForm) -> DomainResult<UserAggr> {
        let mut state = self.state.lock().unwrap();

        // Check uniqueness constraints.
        if state.users.iter().any(|u| u.id == form.id) {
            return Err(DomainError::expected_conflict(trl("error-already-exists")));
        }
        if state.users.iter().any(|u| u.qid == form.qid) {
            return Err(DomainError::expected_conflict(trl("error-already-exists")));
        }
        if state.users.iter().any(|u| u.nickname == form.nickname) {
            return Err(DomainError::expected_conflict(trl("error-already-exists")));
        }

        // Build the user aggregate from the form.
        let now = OffsetDateTime::now_utc();
        let user = UserAggr {
            id: form.id.clone(),
            nickname: form.nickname.clone(),
            qid: form.qid.clone(),
            is_sadmin: false,
            avatar_key: None,
            avatar_uploaded: false,
            avatar_version: 0,
            last_active_at: now,
            created_at: now,
            updated_at: now,
        };

        let credential = UserCredential {
            user_id: form.id.clone(),
            password_hash: form.password_hash.clone(),
        };

        state.users.push(user.clone());
        state.credentials.push(credential);

        Ok(user)
    }

    async fn update_info(&mut self, update: &UserInfoUpdate) -> DomainResult<UserAggr> {
        let mut state = self.state.lock().unwrap();

        let user = state
            .users
            .iter_mut()
            .find(|u| u.id == update.id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")))?;

        user.nickname = update.nickname.clone();
        user.qid = update.qid.clone();
        user.updated_at = OffsetDateTime::now_utc();

        Ok(user.clone())
    }

    async fn touch_last_active(&mut self, id: &str) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();

        let user = state
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")))?;

        user.last_active_at = OffsetDateTime::now_utc();
        user.updated_at = OffsetDateTime::now_utc();

        Ok(())
    }

    async fn get_by_id_excluded(&mut self, id: &str) -> DomainResult<UserAggr> {
        let state = self.state.lock().unwrap();
        state
            .users
            .iter()
            .find(|u| u.id == id)
            .cloned()
            .ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")))
    }

    async fn reserve_avatar(
        &mut self,
        id: &str,
        file_extension: &str,
    ) -> DomainResult<UserAvatarReservation> {
        let mut state = self.state.lock().unwrap();

        let user = state
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")))?;

        let avatar_version = user.avatar_version + 1;
        let object_key = UserAggr::generate_avatar_key(id, avatar_version, file_extension);
        let previous_object_key = user.avatar_key.clone();

        user.avatar_key = Some(object_key.clone());
        user.avatar_uploaded = false;
        user.avatar_version = avatar_version;
        user.updated_at = OffsetDateTime::now_utc();

        Ok(UserAvatarReservation {
            object_key,
            previous_object_key,
            avatar_version,
        })
    }

    async fn mark_avatar_uploaded(&mut self, id: &str, avatar_version: i64) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();

        let user = state
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")))?;

        if user.avatar_version != avatar_version {
            return Err(DomainError::expected_argument(trl(
                "error-stale-avatar-upload",
            )));
        }

        if user.avatar_uploaded {
            return Ok(());
        }

        user.avatar_uploaded = true;
        user.updated_at = OffsetDateTime::now_utc();

        Ok(())
    }

    async fn delete(&mut self, id: &str) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();

        let pos = state
            .users
            .iter()
            .position(|u| u.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")))?;
        state.users.remove(pos);

        // Remove credentials linked to this user.
        state.credentials.retain(|c| c.user_id != id);

        Ok(())
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
    // update_changes_fields_and_is_readable_after_commit(UserQueryTransactional::update_info)(positive): updates should persist nickname and qid.
    // update_missing_user_returns_expected_error(UserQueryTransactional::update_info)(negative): updating a non-existent user should fail.
    // reserve_avatar_sets_key_and_version(UserQueryTransactional::reserve_avatar)(positive): reserve should set the avatar key and increment version.
    // reserve_avatar_missing_user_returns_expected_error(UserQueryTransactional::reserve_avatar)(negative): reserving for a missing user should fail.
    // mark_avatar_uploaded_sets_flag(UserQueryTransactional::mark_avatar_uploaded)(positive): marking should set avatar_uploaded to true.
    // mark_avatar_uploaded_missing_user_returns_expected_error(UserQueryTransactional::mark_avatar_uploaded)(negative): marking for a missing user should fail.
    // touch_last_active_updates_timestamp(UserQuery::touch_last_active)(positive): touching should update last_active_at.
    // touch_last_active_missing_user_returns_expected_error(UserQuery::touch_last_active)(negative): touching for a missing user should fail.
    // delete_removes_user_and_credentials(UserQueryTransactional::delete)(positive): deleting a user should remove the user and its credentials.
    // delete_missing_returns_error(UserQueryTransactional::delete)(negative): deleting a missing user should fail.

    use futures_util::FutureExt as _;

    use time::OffsetDateTime;

    use crate::domain::model::aggr::user::{UserAggr, UserCredential, UserForm, UserInfoUpdate};
    use crate::domain::query::Transactional;
    use crate::domain::query::user::{UserQuery, UserQueryTransactional};
    use crate::infra::query::memory_mock::MemoryMockQuery;
    use crate::test_util::{is_expected_argument, is_expected_conflict};

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
            avatar_key: None,
            avatar_uploaded: false,
            avatar_version: 0,
            last_active_at: n,
            created_at: n,
            updated_at: n,
        }
    }

    fn make_credential(user_id: &str) -> UserCredential {
        UserCredential {
            user_id: user_id.into(),
            password_hash: "hashed-pw".into(),
        }
    }

    #[tokio::test]
    async fn find_by_id_after_seed() {
        let mock = MemoryMockQuery::new();
        mock.seed_user(
            make_user("user-1", "qid-1", "nick"),
            make_credential("user-1"),
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
            make_credential("user-1"),
        );

        let cred = UserQuery::get_credentials_by_qid(&mock, "qid-1")
            .await
            .unwrap();
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
            async move {
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
            }
            .boxed()
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
            async move {
                let form = UserForm::new(
                    UserAggr::generate_id(),
                    "dup-qid".into(),
                    "nick-1".into(),
                    "pw".into(),
                );
                UserQueryTransactional::create(txn, &form).await.unwrap();
                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    let form = UserForm::new(
                        UserAggr::generate_id(),
                        "dup-qid".into(),
                        "nick-2".into(),
                        "pw".into(),
                    );
                    UserQueryTransactional::create(txn, &form).await
                }
                .boxed()
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
            async move {
                let form = UserForm::new(
                    UserAggr::generate_id(),
                    "qid-1".into(),
                    "dup-nick".into(),
                    "pw".into(),
                );
                UserQueryTransactional::create(txn, &form).await.unwrap();
                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    let form = UserForm::new(
                        UserAggr::generate_id(),
                        "qid-2".into(),
                        "dup-nick".into(),
                        "pw".into(),
                    );
                    UserQueryTransactional::create(txn, &form).await
                }
                .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_conflict(&err));
    }

    #[tokio::test]
    async fn update_changes_fields_and_is_readable_after_commit() {
        let mock = MemoryMockQuery::new();
        mock.seed_user(
            make_user("user-1", "qid-1", "old-nick"),
            make_credential("user-1"),
        );

        mock.transaction_scoped(|txn| {
            async move {
                let update = UserInfoUpdate {
                    id: "user-1".into(),
                    qid: "new-qid".into(),
                    nickname: "new-nick".into(),
                };
                let updated = UserQueryTransactional::update_info(txn, &update)
                    .await
                    .unwrap();
                assert_eq!(updated.qid, "new-qid");
                assert_eq!(updated.nickname, "new-nick");
                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();

        let found = UserQuery::get_by_id(&mock, "user-1").await.unwrap();
        assert_eq!(found.qid, "new-qid");
        assert_eq!(found.nickname, "new-nick");
    }

    #[tokio::test]
    async fn update_missing_user_returns_expected_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    let update = UserInfoUpdate {
                        id: "nonexistent".into(),
                        qid: "q".into(),
                        nickname: "n".into(),
                    };
                    UserQueryTransactional::update_info(txn, &update).await
                }
                .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn reserve_avatar_sets_key_and_version() {
        let mock = MemoryMockQuery::new();
        mock.seed_user(
            make_user("user-1", "qid-1", "nick"),
            make_credential("user-1"),
        );

        let reservation = mock
            .transaction_scoped(|txn| {
                async move { UserQueryTransactional::reserve_avatar(txn, "user-1", "png").await }
                    .boxed()
            })
            .await
            .unwrap();

        let found = UserQuery::get_by_id(&mock, "user-1").await.unwrap();
        assert_eq!(reservation.avatar_version, 1);
        assert_eq!(found.avatar_key, Some("user_avatar/user-1-1.png".into()));
        assert_eq!(found.avatar_version, 1);
    }

    #[tokio::test]
    async fn reserve_avatar_missing_user_returns_expected_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    UserQueryTransactional::reserve_avatar(txn, "nonexistent", "png").await
                }
                .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn mark_avatar_uploaded_sets_flag() {
        let mock = MemoryMockQuery::new();
        mock.seed_user(
            make_user("user-1", "qid-1", "nick"),
            make_credential("user-1"),
        );

        mock.transaction_scoped(|txn| {
            async move {
                UserQueryTransactional::reserve_avatar(txn, "user-1", "png").await?;
                UserQueryTransactional::mark_avatar_uploaded(txn, "user-1", 1).await
            }
            .boxed()
        })
        .await
        .unwrap();

        let found = UserQuery::get_by_id(&mock, "user-1").await.unwrap();
        assert!(found.avatar_uploaded);
    }

    #[tokio::test]
    async fn mark_avatar_uploaded_missing_user_returns_expected_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    UserQueryTransactional::mark_avatar_uploaded(txn, "nonexistent", 1).await
                }
                .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn touch_last_active_updates_timestamp() {
        let mock = MemoryMockQuery::new();
        mock.seed_user(
            make_user("user-1", "qid-1", "nick"),
            make_credential("user-1"),
        );

        let before = UserQuery::get_by_id(&mock, "user-1")
            .await
            .unwrap()
            .last_active_at;

        tokio::time::sleep(std::time::Duration::from_millis(5)).await;

        mock.transaction_scoped(|txn| {
            async move { UserQueryTransactional::touch_last_active(txn, "user-1").await }.boxed()
        })
        .await
        .unwrap();

        let after = UserQuery::get_by_id(&mock, "user-1")
            .await
            .unwrap()
            .last_active_at;
        assert!(after > before);
    }

    #[tokio::test]
    async fn touch_last_active_missing_user_returns_expected_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move { UserQueryTransactional::touch_last_active(txn, "nonexistent").await }
                    .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn delete_removes_user_and_credentials_without_avatar() {
        let mock = MemoryMockQuery::new();
        mock.seed_user(
            make_user("user-1", "qid-1", "nick"),
            make_credential("user-1"),
        );

        mock.transaction_scoped(|txn| {
            async move { UserQueryTransactional::delete(txn, "user-1").await }.boxed()
        })
        .await
        .unwrap();

        let snapshot = mock.snapshot();
        assert!(snapshot.users.is_empty());
        assert!(snapshot.credentials.is_empty());
    }

    #[tokio::test]
    async fn delete_missing_user_returns_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move { UserQueryTransactional::delete(txn, "nonexistent").await }.boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }
}
