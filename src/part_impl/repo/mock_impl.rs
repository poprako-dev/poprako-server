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

use crate::model::announcement::AnnouncementInfo;
use crate::model::assignment::AssignmentInfo;
use crate::model::assignment_invitation::AssignmentInvitationInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::comic_archive::ComicArchiveRecord;
use crate::model::comment::CommentInfo;
use crate::model::member::MemberInfo;
use crate::model::member_invitation::MemberInvitationInfo;
use crate::model::page::PageInfo;
use crate::model::system_mail::SystemMailInfo;
use crate::model::team::TeamInfo;
use crate::model::unit::UnitInfo;
use crate::model::user::{UserCredential, UserInfo};
use crate::model::workset::WorksetInfo;
use crate::part::effect::event::Event;
use crate::part_impl::prom::mock_impl::MockPromRecord;
use crate::result::{ExpectedVariant, RegularError};
use crate::util::DeriveTransactional;

/// In-memory state holding all mock repository records.
#[cfg_attr(test, derive(Clone, Default))]
pub struct MockState {
    pub users: Vec<UserInfo>,
    pub credentials: Vec<UserCredential>,
    pub announcements: Vec<AnnouncementInfo>,
    pub comments: Vec<CommentInfo>,
    pub teams: Vec<TeamInfo>,
    pub members: Vec<MemberInfo>,
    pub member_invitations: Vec<MemberInvitationInfo>,
    pub worksets: Vec<WorksetInfo>,
    pub comics: Vec<ComicInfo>,
    pub chapters: Vec<ChapterInfo>,
    pub assignments: Vec<AssignmentInfo>,
    pub assignment_invitations: Vec<AssignmentInvitationInfo>,
    pub pages: Vec<PageInfo>,
    pub units: Vec<UnitInfo>,
    pub system_mails: Vec<SystemMailInfo>,
    pub archived_comics: Vec<ComicArchiveRecord>,
    pub archived_chapters: Vec<ComicArchiveRecord>,
    pub archived_translations: Vec<ComicArchiveRecord>,
    pub prom_records: Vec<MockPromRecord>,
    pub deleted_image_keys: Vec<String>,
}

/// A point-in-time copy of [MockState] for assertions after a transaction.
#[cfg_attr(test, derive(Clone))]
pub struct MockSnapshot {
    pub users: Vec<UserInfo>,
    pub credentials: Vec<UserCredential>,
    pub announcements: Vec<AnnouncementInfo>,
    pub comments: Vec<CommentInfo>,
    pub teams: Vec<TeamInfo>,
    pub members: Vec<MemberInfo>,
    pub member_invitations: Vec<MemberInvitationInfo>,
    pub worksets: Vec<WorksetInfo>,
    pub comics: Vec<ComicInfo>,
    pub chapters: Vec<ChapterInfo>,
    pub assignments: Vec<AssignmentInfo>,
    pub assignment_invitations: Vec<AssignmentInvitationInfo>,
    pub pages: Vec<PageInfo>,
    pub units: Vec<UnitInfo>,
    pub system_mails: Vec<SystemMailInfo>,
    pub archived_comics: Vec<ComicArchiveRecord>,
    pub archived_chapters: Vec<ComicArchiveRecord>,
    pub archived_translations: Vec<ComicArchiveRecord>,
    pub prom_records: Vec<MockPromRecord>,
    pub deleted_image_keys: Vec<String>,
}

impl From<MockState> for MockSnapshot {
    fn from(state: MockState) -> Self {
        Self {
            users: state.users,
            credentials: state.credentials,
            announcements: state.announcements,
            comments: state.comments,
            teams: state.teams,
            members: state.members,
            member_invitations: state.member_invitations,
            worksets: state.worksets,
            comics: state.comics,
            chapters: state.chapters,
            assignments: state.assignments,
            assignment_invitations: state.assignment_invitations,
            pages: state.pages,
            units: state.units,
            system_mails: state.system_mails,
            archived_comics: state.archived_comics,
            archived_chapters: state.archived_chapters,
            archived_translations: state.archived_translations,
            prom_records: state.prom_records,
            deleted_image_keys: state.deleted_image_keys,
        }
    }
}

/// The transactional context passed to [Drive::with_context] calls,
/// providing mutable access to the mock state during a simulated transaction.
pub struct MockContext {
    pub state: MockState,
    pub archive_commit_failure: bool,
}

/// Flag toggles that inject controlled failure modes into mock opers.
#[cfg_attr(test, derive(Clone, Default))]
pub struct MockFlags {
    pub token_failure: bool,
    pub image_get_failure: bool,
    pub image_put_failure: bool,

    pub image_head_failure: bool,
    pub image_head_absent: bool,

    pub image_delete_failure: bool,
    pub archive_commit_failure: bool,
}

/// The top-level mock driver implementing [DeriveTransactional] and [Drive].
/// Wraps shared mutable state, failure flags, and an event buffer behind
/// `Arc<Mutex<...>>` for concurrent test access.
#[cfg_attr(test, derive(Clone, Default))]
pub struct Mock {
    pub state: Arc<Mutex<MockState>>,
    pub flags: Arc<Mutex<MockFlags>>,
    pub events: Arc<Mutex<Vec<Event>>>,
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

    /// Seed an announcement directly into the mock state.
    pub fn seed_announcement(&self, announcement: AnnouncementInfo) {
        self.state.lock().unwrap().announcements.push(announcement);
    }

    /// Seed a comment directly into the mock state.
    pub fn seed_comment(&self, comment: CommentInfo) {
        self.state.lock().unwrap().comments.push(comment);
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
    pub fn seed_member_invitation(
        &self,
        member_invitation: MemberInvitationInfo,
    ) {
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

    /// Seed a chapter directly into the mock state.
    pub fn seed_chapter(&self, chapter: ChapterInfo) {
        self.state.lock().unwrap().chapters.push(chapter);
    }

    /// Seed an assignment directly into the mock state.
    pub fn seed_assignment(&self, assignment: AssignmentInfo) {
        self.state.lock().unwrap().assignments.push(assignment);
    }

    /// Seed an assignment invitation directly into the mock state.
    pub fn seed_assignment_invitation(
        &self,
        assignment_invitation: AssignmentInvitationInfo,
    ) {
        self.state
            .lock()
            .unwrap()
            .assignment_invitations
            .push(assignment_invitation);
    }

    /// Seed a page directly into the mock state.
    pub fn seed_page(&self, page: PageInfo) {
        self.state.lock().unwrap().pages.push(page);
    }

    /// Seed a unit directly into the mock state.
    pub fn seed_unit(&self, unit: UnitInfo) {
        self.state.lock().unwrap().units.push(unit);
    }

    /// Seed a system mail directly into the mock state.
    pub fn seed_system_mail(&self, system_mail: SystemMailInfo) {
        self.state.lock().unwrap().system_mails.push(system_mail);
    }

    /// Return a point-in-time copy of the current mock state for assertion.
    pub fn snapshot(&self) -> MockSnapshot {
        self.state.lock().unwrap().clone().into()
    }

    /// Enable token authentication failures for subsequent opers.
    pub fn with_token_failure(self) -> Self {
        self.flags.lock().unwrap().token_failure = true;
        self
    }

    /// Enable image retrieval failures for subsequent opers.
    pub fn with_image_get_failure(self) -> Self {
        self.flags.lock().unwrap().image_get_failure = true;
        self
    }

    /// Enable image storage failures for subsequent opers.
    pub fn with_image_put_failure(self) -> Self {
        self.flags.lock().unwrap().image_put_failure = true;
        self
    }

    /// Enable head-object failures for subsequent opers.
    pub fn with_image_head_failure(self) -> Self {
        self.flags.lock().unwrap().image_head_failure = true;
        self
    }

    /// Report objects as absent for subsequent head-object opers.
    pub fn with_image_head_absent(self) -> Self {
        self.flags.lock().unwrap().image_head_absent = true;
        self
    }

    /// Enable delete-object failures for subsequent opers.
    pub fn with_image_delete_failure(self) -> Self {
        self.flags.lock().unwrap().image_delete_failure = true;
        self
    }

    /// Fail archive persistence before a transaction can commit.
    pub fn with_archive_commit_failure(self) -> Self {
        self.flags.lock().unwrap().archive_commit_failure = true;
        self
    }

    /// Return the number of events emitted so far.
    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    /// Drain and return all accumulated events, clearing the buffer.
    pub fn drain_events(&self) -> Vec<Event> {
        std::mem::take(&mut *self.events.lock().unwrap())
    }
}

/// A zero-sized marker representing a live transaction handle in the mock driver.
pub struct MockTransactional;

#[async_trait]
impl DeriveTransactional for Mock {
    type Transactional = MockTransactional;

    async fn derive_transactional(&self) -> Self::Transactional {
        MockTransactional
    }
}

#[async_trait]
impl Drive<MockContext> for Mock {
    type Error = RegularError;

    async fn with_context<T, E, F>(
        &self,
        f: F,
    ) -> Result<T, DriveError<E, Self::Error>>
    where
        T: Send,
        E: Send,
        for<'c> F: AsyncFnOnce(&'c mut MockContext) -> Result<T, E>
            + AsyncFnMark<&'c mut MockContext, Result<T, E>, Fut: Send>
            + Send,
    {
        let mut context = MockContext {
            state: self.state.lock().unwrap().clone(),
            archive_commit_failure: self
                .flags
                .lock()
                .unwrap()
                .archive_commit_failure,
        };

        match f(&mut context).await {
            Ok(value) => {
                *self.state.lock().unwrap() = context.state;
                Ok(value)
            }
            Err(err) => Err(DriveError::Advance(err)),
        }
    }
}

/// Build an expected-args [RootError] with a translated message.
pub fn expected(message: &str) -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Args,
        message: trl(message),
    }
}

/// Build an unrecoverable [RootError] with the given message.
pub fn unrecoverable(message: &str) -> RegularError {
    RegularError::Unrecoverable {
        message: message.into(),
    }
}

/// Return the current UTC timestamp.
pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

/// Mock implementations for announcement repository opers.
pub mod announcement;

/// Mock implementations for assignment repository opers.
pub mod assignment;

/// Mock implementations for assignment invitation repository opers.
pub mod assignment_invitation;

/// Mock implementations for chapter repository operations.
pub mod chapter;

/// Mock implementations for comment repository opers.
pub mod comment;

pub mod comic;
/// Mock implementations for immutable comic archive repository operations.
pub mod comic_archive;

/// Mock implementations for member repository opers.
pub mod member;

/// Mock implementations for member invitation repository opers.
pub mod member_invitation;

/// Mock implementations for page repository opers.
pub mod page;

/// Mock implementations for system mail repository opers.
pub mod system_mail;

/// Mock implementations for team repository opers.
pub mod team;

/// Mock implementations for unit repository opers.
pub mod unit;

/// Mock implementations for user repository opers.
pub mod user;

/// Mock implementations for workset repository opers.
pub mod workset;

// execute_reads_seeded_user(Execute<UserStep::get_info_by_id>)(positive): a seeded user should be readable outside a transaction.
// transaction_commits_repo_and_prom(Drive::with_context)(positive): successful transactions should commit repo and prom state together.
// transaction_rolls_back_repo_and_prom(Drive::with_context)(negative): failed transactions should discard repo and prom state together.

use poprako_transactional::advance::Advance;

use crate::model::member::MemberForm;
use crate::part::prom::task::{ImageKind, ImageTask};
use crate::part::prom::{Payload, PromStep};
use crate::part::repo::step::member::MemberStep;
use crate::part::repo::step::user::UserStep;
use crate::part::shared::execute::Execute;
use crate::value::role::{RoleField, RoleMask};

/// Build a minimal `UserInfo` for test seeding.
fn user(id: &str) -> UserInfo {
    //
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

/// Mock helper that verifies a seeded user is readable outside a transaction.
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

    let found =
        Execute::execute(&mock, &UserStep::get_info_by_id("user-1")).await;
    assert!(found.is_ok());
    let found = found.ok().unwrap();

    assert_eq!(found.id, "user-1");
}

/// Mock helper that verifies successful transactions commit repo and prom state.
#[tokio::test]
async fn transaction_commits_repo_and_prom() {
    let mock = Mock::new();
    let member_form = MemberForm {
        id: "member-1".into(),
        user_id: "user-1".into(),
        user_nickname: "nick".into(),
        team_id: "team-1".into(),
        roles: RoleMask::from(RoleField::RAW_PROVIDER),
    };
    let visible_at = now();

    assert!(
        Drive::with_context(&mock, async move |context| {
            let transactional = MockTransactional;
            Advance::advance(
                &transactional,
                context,
                &MemberStep::create(&member_form),
            )
            .await?;
            Advance::advance(
                &transactional,
                context,
                &PromStep::append(
                    "prom-1",
                    "image",
                    Payload::Image(ImageTask::CheckUploaded {
                        kind: ImageKind::UserAvatar,
                        resource_id: "user-1",
                        object_key: "key",
                        image_version: 1,
                    }),
                    &visible_at,
                ),
            )
            .await?;
            Ok::<(), RegularError>(())
        })
        .await
        .is_ok()
    );

    let snapshot = mock.snapshot();
    assert_eq!(snapshot.members.len(), 1);
    assert_eq!(snapshot.prom_records.len(), 1);
}

/// Mock helper that verifies failed transactions discard repo and prom state.
#[tokio::test]
async fn transaction_rolls_back_repo_and_prom() {
    let mock = Mock::new();
    let member_form = MemberForm {
        id: "member-1".into(),
        user_id: "user-1".into(),
        user_nickname: "nick".into(),
        team_id: "team-1".into(),
        roles: RoleMask::from(RoleField::RAW_PROVIDER),
    };
    let visible_at = now();

    let err = Drive::with_context(&mock, async move |context| {
        let transactional = MockTransactional;
        Advance::advance(
            &transactional,
            context,
            &MemberStep::create(&member_form),
        )
        .await?;
        Advance::advance(
            &transactional,
            context,
            &PromStep::append(
                "prom-1",
                "image",
                Payload::Image(ImageTask::Delete { object_key: "key" }),
                &visible_at,
            ),
        )
        .await?;
        Err::<(), _>(unrecoverable(
            "[transaction_rolls_back_repo_and_prom] fail",
        ))
    })
    .await
    .err()
    .unwrap();

    assert!(matches!(
        err,
        DriveError::Advance(RegularError::Unrecoverable { .. })
    ));
    let snapshot = mock.snapshot();
    assert!(snapshot.members.is_empty());
    assert!(snapshot.prom_records.is_empty());
}
