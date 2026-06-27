//! In-memory mock implementations of repository and prom ports for testing.
//! Provides [`Mock`], [`MockState`], and related types to simulate storage
//! and side-effect behavior without external dependencies.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use time::OffsetDateTime;

use poprako_transactional::drive::Drive;
use poprako_transactional::drive::result::Error as DriveError;
use poprako_transactional::util::AsyncFnMark;
use poprako_util::i18n::trl;

use crate::model::comic::ComicInfo;
use crate::model::member::MemberInfo;
use crate::model::member_invitation::MemberInvitationInfo;
use crate::model::system_mail::SystemMailInfo;
use crate::model::team::TeamInfo;
use crate::model::user::{UserCredential, UserInfo};
use crate::model::workset::WorksetInfo;
use crate::result::{ExpectedVariant, RootError};
use crate::util::DeriveTransactional;

/// In-memory state holding all mock repository records.
#[cfg_attr(test, derive(Clone, Default))]
pub struct MockState {
    pub users: Vec<UserInfo>,
    pub credentials: Vec<UserCredential>,
    pub teams: Vec<TeamInfo>,
    pub members: Vec<MemberInfo>,
    pub member_invitations: Vec<MemberInvitationInfo>,
    pub worksets: Vec<WorksetInfo>,
    pub comics: Vec<ComicInfo>,
    pub system_mails: Vec<SystemMailInfo>,
    pub prom_records: Vec<super::prom_mock::MockPromRecord>,
}

/// A point-in-time copy of [MockState] for assertions after a transaction.
#[cfg_attr(test, derive(Clone))]
pub struct MockSnapshot {
    pub users: Vec<UserInfo>,
    pub credentials: Vec<UserCredential>,
    pub teams: Vec<TeamInfo>,
    pub members: Vec<MemberInfo>,
    pub member_invitations: Vec<MemberInvitationInfo>,
    pub worksets: Vec<WorksetInfo>,
    pub comics: Vec<ComicInfo>,
    pub system_mails: Vec<SystemMailInfo>,
    pub prom_records: Vec<super::prom_mock::MockPromRecord>,
}

impl From<MockState> for MockSnapshot {
    fn from(state: MockState) -> Self {
        Self {
            users: state.users,
            credentials: state.credentials,
            teams: state.teams,
            members: state.members,
            member_invitations: state.member_invitations,
            worksets: state.worksets,
            comics: state.comics,
            system_mails: state.system_mails,
            prom_records: state.prom_records,
        }
    }
}

/// The transactional context passed to [Drive::with_context] closures,
/// providing mutable access to the mock state during a simulated transaction.
pub struct MockContext {
    pub state: MockState,
}

/// Flag toggles that inject controlled failure modes into mock operations.
#[cfg_attr(test, derive(Clone, Default))]
pub struct MockFlags {
    pub token_failure: bool,
    pub image_get_failure: bool,
    pub image_put_failure: bool,
}

/// The top-level mock driver implementing [DeriveTransactional] and [Drive].
/// Wraps shared mutable state, failure flags, and an event buffer behind
/// `Arc<Mutex<...>>` for concurrent test access.
#[cfg_attr(test, derive(Clone, Default))]
pub struct Mock {
    pub state: Arc<Mutex<MockState>>,
    pub flags: Arc<Mutex<MockFlags>>,
    pub events: Arc<Mutex<Vec<crate::part::effect::event::Event>>>,
}

impl Mock {
    /// Create a new mock with empty state and no failure flags.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed a user and its credential directly into the mock state.
    pub fn seed_user(&self, user: UserInfo, credential: UserCredential) {
        let mut state = self.state.lock().unwrap();
        state.users.push(user);
        state.credentials.push(credential);
    }

    /// Seed a team directly into the mock state.
    pub fn seed_team(&self, team: TeamInfo) {
        self.state.lock().unwrap().teams.push(team);
    }

    /// Seed a member directly into the mock state.
    pub fn seed_member(&self, member: MemberInfo) {
        self.state.lock().unwrap().members.push(member);
    }

    /// Seed a member invitation directly into the mock state.
    pub fn seed_member_invitation(&self, member_invitation: MemberInvitationInfo) {
        self.state
            .lock()
            .unwrap()
            .member_invitations
            .push(member_invitation);
    }

    /// Seed a workset directly into the mock state.
    pub fn seed_workset(&self, workset: WorksetInfo) {
        self.state.lock().unwrap().worksets.push(workset);
    }

    /// Seed a comic directly into the mock state.
    pub fn seed_comic(&self, comic: ComicInfo) {
        self.state.lock().unwrap().comics.push(comic);
    }

    /// Seed a system mail directly into the mock state.
    pub fn seed_system_mail(&self, system_mail: SystemMailInfo) {
        self.state.lock().unwrap().system_mails.push(system_mail);
    }

    /// Return a point-in-time copy of the current mock state for assertion.
    pub fn snapshot(&self) -> MockSnapshot {
        self.state.lock().unwrap().clone().into()
    }

    /// Enable token authentication failures for subsequent operations.
    pub fn with_token_failure(self) -> Self {
        self.flags.lock().unwrap().token_failure = true;
        self
    }

    /// Enable image retrieval failures for subsequent operations.
    pub fn with_image_get_failure(self) -> Self {
        self.flags.lock().unwrap().image_get_failure = true;
        self
    }

    /// Enable image storage failures for subsequent operations.
    pub fn with_image_put_failure(self) -> Self {
        self.flags.lock().unwrap().image_put_failure = true;
        self
    }

    /// Return the number of events emitted so far.
    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    /// Drain and return all accumulated events, clearing the buffer.
    pub fn drain_events(&self) -> Vec<crate::part::effect::event::Event> {
        std::mem::take(&mut *self.events.lock().unwrap())
    }
}

/// A zero-sized marker representing a live transaction handle in the mock driver.
pub struct MockTransactional;

#[async_trait]
impl DeriveTransactional for Mock {
    type Transactional = MockTransactional;

    async fn transactional(&self) -> Self::Transactional {
        MockTransactional
    }
}

#[async_trait]
impl Drive<MockContext> for Mock {
    type Error = RootError;

    async fn with_context<T, E, F>(&self, f: F) -> Result<T, DriveError<E, Self::Error>>
    where
        T: Send,
        E: Send,
        for<'c> F: AsyncFnOnce(&'c mut MockContext) -> Result<T, E>
            + AsyncFnMark<&'c mut MockContext, Result<T, E>, Fut: Send>
            + Send,
    {
        let mut context = MockContext {
            state: self.state.lock().unwrap().clone(),
        };

        let result = f(&mut context).await;

        match result {
            Ok(value) => {
                *self.state.lock().unwrap() = context.state;
                Ok(value)
            }
            Err(err) => Err(DriveError::Advance(err)),
        }
    }
}

/// Build an expected-args [RootError] with a translated message.
pub(super) fn expected(message: &str) -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::Args,
        message: trl(message),
    }
}

/// Build an unrecoverable [RootError] with the given message.
pub(super) fn unrecoverable(message: &str) -> RootError {
    RootError::Unrecoverable {
        message: message.into(),
    }
}

/// Return the current UTC timestamp.
pub(super) fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

/// Mock implementations for comic repository operations.
pub mod comic;

/// Mock implementations for member repository operations.
pub mod member;

/// Mock implementations for member invitation repository operations.
pub mod member_invitation;

/// Mock implementations for system mail repository operations.
pub mod system_mail;

/// Mock implementations for team repository operations.
pub mod team;

/// Mock implementations for user repository operations.
pub mod user;

/// Mock implementations for workset repository operations.
pub mod workset;

mod tests {
    // execute_reads_seeded_user(Execute<UserStep::get_info_by_id>)(positive): a seeded user should be readable outside a transaction.
    // transaction_commits_repo_and_prom(Drive::with_context)(positive): successful transactions should commit repo and prom state together.
    // transaction_rolls_back_repo_and_prom(Drive::with_context)(negative): failed transactions should discard repo and prom state together.

    use super::*;

    use poprako_transactional::advance::Advance;

    use crate::model::member::MemberForm;
    use crate::model::role::{RoleBit, RoleMask};
    use crate::model::user::{UserCredential, UserInfo};
    use crate::part::prom::intention::{ImageIntention, ImageKind};
    use crate::part::prom::{Payload, PromStep};
    use crate::part::repo::Execute;
    use crate::part::repo::step::member::MemberStep;
    use crate::part::repo::step::user::UserStep;
    use crate::result::accept;

    fn user(id: &str) -> UserInfo {
        let time = now();
        UserInfo {
            id: id.into(),
            qid: "qid".into(),
            nickname: "nick".into(),
            avatar_key: None,
            avatar_uploaded: false,
            avatar_version: 0,
            is_sadmin: false,
            last_active_at: time,
            created_at: time,
            updated_at: time,
        }
    }

    #[tokio::test]
    async fn execute_reads_seeded_user() {
        let mock = Mock::new();
        mock.seed_user(
            user("user-1"),
            UserCredential {
                user_id: "user-1".into(),
                password_hash: "hash".into(),
            },
        );

        let found = Execute::execute(&mock, &UserStep::get_info_by_id("user-1")).await;
        assert!(found.is_ok());
        let found = found.ok().unwrap();

        assert_eq!(found.id, "user-1");
    }

    #[tokio::test]
    async fn transaction_commits_repo_and_prom() {
        let mock = Mock::new();
        let member_form = MemberForm {
            id: "member-1".into(),
            user_id: "user-1".into(),
            user_nickname: "nick".into(),
            team_id: "team-1".into(),
            role_mask: RoleMask::from(RoleBit::RAW_PROVIDER),
        };
        let visible_at = now();

        let result = Drive::with_context(&mock, async move |context| {
            let transactional = MockTransactional;
            Advance::advance(&transactional, context, &MemberStep::create(&member_form)).await?;
            Advance::advance(
                &transactional,
                context,
                &PromStep::append(
                    "prom-1",
                    "image",
                    Payload::Image(ImageIntention::CheckUploaded {
                        kind: ImageKind::UserAvatar,
                        resource_id: "user-1".into(),
                        object_key: "key".into(),
                        image_version: 1,
                    }),
                    &visible_at,
                ),
            )
            .await?;
            accept(())
        })
        .await;
        assert!(result.is_ok());

        let snapshot = mock.snapshot();
        assert_eq!(snapshot.members.len(), 1);
        assert_eq!(snapshot.prom_records.len(), 1);
    }

    #[tokio::test]
    async fn transaction_rolls_back_repo_and_prom() {
        let mock = Mock::new();
        let member_form = MemberForm {
            id: "member-1".into(),
            user_id: "user-1".into(),
            user_nickname: "nick".into(),
            team_id: "team-1".into(),
            role_mask: RoleMask::from(RoleBit::RAW_PROVIDER),
        };
        let visible_at = now();

        let err = Drive::with_context(&mock, async move |context| {
            let transactional = MockTransactional;
            Advance::advance(&transactional, context, &MemberStep::create(&member_form)).await?;
            Advance::advance(
                &transactional,
                context,
                &PromStep::append(
                    "prom-1",
                    "image",
                    Payload::Image(ImageIntention::Delete {
                        object_key: "key".into(),
                    }),
                    &visible_at,
                ),
            )
            .await?;
            Err::<(), RootError>(unrecoverable("[transaction_rolls_back_repo_and_prom] fail"))
        })
        .await
        .err()
        .unwrap();

        assert!(matches!(
            err,
            DriveError::Advance(RootError::Unrecoverable { .. })
        ));
        let snapshot = mock.snapshot();
        assert!(snapshot.members.is_empty());
        assert!(snapshot.prom_records.is_empty());
    }
}
