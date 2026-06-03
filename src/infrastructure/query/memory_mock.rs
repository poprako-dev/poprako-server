pub mod member;
pub mod member_invitation;
pub mod system_mail;
pub mod team;
pub mod user;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_util::future::BoxFuture;

use crate::domain::model::aggregate::member::MemberAggr;
use crate::domain::model::aggregate::member_invitation::MemberInvitationAggr;
use crate::domain::model::aggregate::system_mail::SystemMailAggr;
use crate::domain::model::aggregate::team::TeamAggr;
use crate::domain::model::aggregate::user::{UserAggr, UserCredential};
use crate::domain::query::Transactional;
use crate::domain::result::DomainResult;

// ── Shared in-memory state ─────────────────────────────────────────────────

/// In-memory store for mock query implementations.
///
/// All collections hold owned aggregate values.  Lookups are linear scans;
/// the data volume in tests is small enough that this has no measurable cost.
#[derive(Default, Clone)]
pub struct MemoryMockState {
    pub users: Vec<UserAggr>,
    pub credentials: Vec<UserCredential>,
    pub teams: Vec<TeamAggr>,
    pub members: Vec<MemberAggr>,
    pub member_invitations: Vec<MemberInvitationAggr>,
    pub system_mails: Vec<SystemMailAggr>,
}

// ── Non-transactional query handle ─────────────────────────────────────────

/// Query handle backed by in-memory state.
///
/// Every call to [`MemoryMockQuery::new`] produces an independent state
/// instance, so tests can run in parallel without interference.
pub struct MemoryMockQuery {
    state: Arc<Mutex<MemoryMockState>>,
}

impl MemoryMockQuery {
    /// Creates a new mock query with an empty state.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MemoryMockState::default())),
        }
    }

    /// Pre-populates the store with a user and its credential.
    pub fn seed_user(&self, user: UserAggr, credential: UserCredential) {
        let mut state = self.state.lock().unwrap();
        state.users.push(user);
        state.credentials.push(credential);
    }

    /// Pre-populates the store with a team.
    pub fn seed_team(&self, team: TeamAggr) {
        let mut state = self.state.lock().unwrap();
        state.teams.push(team);
    }

    /// Pre-populates the store with a member invitation.
    pub fn seed_member_invitation(&self, invitation: MemberInvitationAggr) {
        let mut state = self.state.lock().unwrap();
        state.member_invitations.push(invitation);
    }

    /// Returns a snapshot of the current state for test assertions.
    pub fn snapshot(&self) -> MemoryMockState {
        self.state.lock().unwrap().clone()
    }
}

impl Default for MemoryMockQuery {
    fn default() -> Self {
        Self::new()
    }
}

// ── Transaction-scoped query handle ────────────────────────────────────────

/// Mutable query handle passed to closures inside
/// [`Transactional::transaction_scoped`].
///
/// Writes are applied directly to the shared state and restored from a
/// snapshot if the transaction returns an error.
pub struct MemoryMockQueryTransactional {
    state: Arc<Mutex<MemoryMockState>>,
}

// ── Transactional impl ─────────────────────────────────────────────────────

#[async_trait]
impl Transactional for MemoryMockQuery {
    type Query<'a> = MemoryMockQueryTransactional;

    async fn transaction_scoped<F, T>(&self, f: F) -> DomainResult<T>
    where
        T: Send,
        F: for<'a> FnOnce(&'a mut Self::Query<'a>) -> BoxFuture<'a, DomainResult<T>> + Send,
    {
        let snapshot = self.state.lock().unwrap().clone();
        let mut query = MemoryMockQueryTransactional {
            state: Arc::clone(&self.state),
        };
        let res = f(&mut query).await;

        if res.is_err() {
            *self.state.lock().unwrap() = snapshot;
        }

        res
    }
}

#[cfg(test)]
mod tests {
    // transaction_scoped_restores_snapshot_on_error(Transactional::transaction_scoped)(negative): transaction errors should restore the pre-transaction snapshot.

    use crate::domain::model::aggregate::user::{UserAggr, UserForm};
    use crate::domain::query::Transactional;
    use crate::domain::query::user::UserQueryTransactional;
    use crate::domain::result::DomainError;
    use crate::infrastructure::query::memory_mock::MemoryMockQuery;

    #[tokio::test]
    async fn transaction_scoped_restores_snapshot_on_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                Box::pin(async move {
                    let form = UserForm::new(
                        UserAggr::generate_id(),
                        "qid-1".into(),
                        "nick-1".into(),
                        "pw".into(),
                    );
                    UserQueryTransactional::create(txn, &form).await?;
                    Err::<(), DomainError>(DomainError::unrecoverable("rollback".into()))
                })
            })
            .await
            .err()
            .unwrap();

        assert!(matches!(err, DomainError::Unrecoverable { .. }));
        assert!(mock.snapshot().users.is_empty());
    }
}
