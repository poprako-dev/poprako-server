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

pub struct MockContext {
    pub state: MockState,
}

#[cfg_attr(test, derive(Clone, Default))]
pub struct MockFlags {
    pub token_failure: bool,
    pub image_get_failure: bool,
    pub image_put_failure: bool,
}

#[cfg_attr(test, derive(Clone, Default))]
pub struct Mock {
    pub state: Arc<Mutex<MockState>>,
    pub flags: Arc<Mutex<MockFlags>>,
    pub events: Arc<Mutex<Vec<crate::part::effect::event::Event>>>,
}

impl Mock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed_user(&self, user: UserInfo, credential: UserCredential) {
        let mut state = self.state.lock().unwrap();
        state.users.push(user);
        state.credentials.push(credential);
    }

    pub fn seed_team(&self, team: TeamInfo) {
        self.state.lock().unwrap().teams.push(team);
    }

    pub fn seed_member(&self, member: MemberInfo) {
        self.state.lock().unwrap().members.push(member);
    }

    pub fn seed_member_invitation(&self, member_invitation: MemberInvitationInfo) {
        self.state
            .lock()
            .unwrap()
            .member_invitations
            .push(member_invitation);
    }

    pub fn seed_workset(&self, workset: WorksetInfo) {
        self.state.lock().unwrap().worksets.push(workset);
    }

    pub fn seed_comic(&self, comic: ComicInfo) {
        self.state.lock().unwrap().comics.push(comic);
    }

    pub fn seed_system_mail(&self, system_mail: SystemMailInfo) {
        self.state.lock().unwrap().system_mails.push(system_mail);
    }

    pub fn snapshot(&self) -> MockSnapshot {
        self.state.lock().unwrap().clone().into()
    }

    pub fn with_token_failure(self) -> Self {
        self.flags.lock().unwrap().token_failure = true;
        self
    }

    pub fn with_image_get_failure(self) -> Self {
        self.flags.lock().unwrap().image_get_failure = true;
        self
    }

    pub fn with_image_put_failure(self) -> Self {
        self.flags.lock().unwrap().image_put_failure = true;
        self
    }

    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    pub fn drain_events(&self) -> Vec<crate::part::effect::event::Event> {
        std::mem::take(&mut *self.events.lock().unwrap())
    }
}

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

pub(super) fn expected(message: &str) -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::Args,
        message: trl(message),
    }
}

pub(super) fn unrecoverable(message: &str) -> RootError {
    RootError::Unrecoverable {
        message: message.into(),
    }
}

pub(super) fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

pub mod comic;
pub mod member;
pub mod member_invitation;
pub mod system_mail;
pub mod team;
pub mod user;
pub mod workset;

mod tests {
    // execute_reads_seeded_user(Execute<UserStep::get_info_by_id>)(positive): a seeded user should be readable outside a transaction.
    // transaction_commits_repo_and_prom(Drive::with_context)(positive): successful transactions should commit repo and prom state together.
    // transaction_rolls_back_repo_and_prom(Drive::with_context)(negative): failed transactions should discard repo and prom state together.

    use super::*;

    use poprako_transactional::advance::Advance;

    use crate::model::member::MemberForm;
    use crate::model::role::RoleMask;
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
        let form = MemberForm {
            id: "member-1".into(),
            user_id: "user-1".into(),
            user_nickname: "nick".into(),
            team_id: "team-1".into(),
            role_mask: RoleMask(1),
        };
        let visible_at = now();

        let result = Drive::with_context(&mock, async move |context| {
            let txn = MockTransactional;
            Advance::advance(&txn, context, &MemberStep::create(&form)).await?;
            Advance::advance(
                &txn,
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
        let form = MemberForm {
            id: "member-1".into(),
            user_id: "user-1".into(),
            user_nickname: "nick".into(),
            team_id: "team-1".into(),
            role_mask: RoleMask(1),
        };
        let visible_at = now();

        let err = Drive::with_context(&mock, async move |context| {
            let txn = MockTransactional;
            Advance::advance(&txn, context, &MemberStep::create(&form)).await?;
            Advance::advance(
                &txn,
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
