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
/// Does not simulate snapshot / rollback / commit — writes are applied
/// directly to the shared state.
pub struct MemoryMockQueryTransactional {
    pub(super) state: Arc<Mutex<MemoryMockState>>,
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
        let mut query = MemoryMockQueryTransactional {
            state: Arc::clone(&self.state),
        };
        f(&mut query).await
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::model::aggregate::member::MemberForm;
    use crate::domain::model::aggregate::member_invitation::MemberInvitationAggr;
    use crate::domain::model::aggregate::user::UserForm;
    use crate::domain::model::value::role::{RoleFlag, RoleMask};
    use crate::domain::query::member::MemberQueryTransactional;
    use crate::domain::query::member_invitation::MemberInvitationQueryTransactional;
    use crate::domain::query::user::UserQueryTransactional;
    use time::OffsetDateTime;

    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn make_invitation(id: &str, code: &str, pending: bool) -> MemberInvitationAggr {
        MemberInvitationAggr::new(
            id.into(),
            "invitor-1".into(),
            None,
            "team-1".into(),
            "invitee-qid".into(),
            code.into(),
            pending,
            RoleMask::from(RoleFlag::Admin),
            now(),
        )
    }

    #[tokio::test]
    async fn transaction_scoped_creates_user_member_marks_invitation_used() {
        let mock = MemoryMockQuery::new();
        mock.seed_member_invitation(make_invitation("inv-1", "CODE-XYZ", true));

        mock.transaction_scoped(|txn| {
            Box::pin(async move {
                // 1. Look up the pending invitation.
                let inv = MemberInvitationQueryTransactional::get_by_code_ex(txn, "CODE-XYZ")
                    .await
                    .unwrap();

                // 2. Create a user.
                let user_form = UserForm::new("qid-txn".into(), "nick-txn".into(), "pw".into());
                let user = UserQueryTransactional::create(txn, &user_form)
                    .await
                    .unwrap();

                // 3. Create a member for the user.
                let member_form = MemberForm::new(
                    user.id.clone(),
                    user.nickname.clone(),
                    inv.team_id.clone(),
                    inv.roles,
                );
                MemberQueryTransactional::create(txn, &member_form)
                    .await
                    .unwrap();

                // 4. Mark the invitation as used.
                MemberInvitationQueryTransactional::mark_pending_as_used(txn, &inv.id)
                    .await
                    .unwrap();

                Ok(())
            })
        })
        .await
        .unwrap();

        let snap = mock.snapshot();

        // User created.
        assert_eq!(snap.users.len(), 1);
        assert_eq!(snap.users[0].qid, "qid-txn");

        // Credential stored.
        assert_eq!(snap.credentials.len(), 1);
        assert_eq!(snap.credentials[0].qid, "qid-txn");

        // Member created.
        assert_eq!(snap.members.len(), 1);
        assert_eq!(snap.members[0].team_id, "team-1");

        // Invitation is no longer pending.
        let inv = snap
            .member_invitations
            .iter()
            .find(|i| i.id == "inv-1")
            .unwrap();
        assert!(!inv.pending);
    }
}
