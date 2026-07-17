//! In-memory repository and prom adapters for tests.

use std::sync::{Arc, Mutex};

use poprako_orchestra::nucl::Error as NuclError;
use poprako_orchestra::{Nucl, Run as _, Step as _};
use poprako_orchestra_extra::prom::oper::Defer;
use poprako_orchestra_extra::prom::task::Task;
use time::OffsetDateTime;

use poprako_util::i18n::trl;

use crate::model::announcement::AnnouncementInfo;
use crate::model::assignment::AssignmentInfo;
use crate::model::assignment_invitation::AssignmentInvitationInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::comic_archive::ComicArchiveRecord;
use crate::model::comment::CommentInfo;
use crate::model::member::{MemberEntry, MemberInfo};
use crate::model::member_invitation::MemberInvitationInfo;
use crate::model::page::PageInfo;
use crate::model::system_mail::SystemMailInfo;
use crate::model::team::TeamInfo;
use crate::model::unit::UnitInfo;
use crate::model::user::{UserCredential, UserInfo};
use crate::model::workset::WorksetInfo;
use crate::part::effect::event::Event;
use crate::part::prom::payload::{Payload, image};
use crate::part::repo::oper::member::CreateMember;
use crate::part::repo::oper::user::GetUserInfo;
use crate::part_impl::prom::mock_impl::MockPromRecord;
use crate::result::{BaseError, ExpectedVariant};
use crate::value::role::{RoleField, RoleMask};

/// Mock implementations for announcement repository opers.
pub mod announcement;
/// Mock implementations for assignment repository opers.
pub mod assignment;
/// Mock implementations for assignment invitation repository opers.
pub mod assignment_invitation;
/// Mock implementations for chapter repository operations.
pub mod chapter;
pub mod comic;
/// Mock implementations for immutable comic archive repository operations.
pub mod comic_archive;
/// Mock implementations for comment repository opers.
pub mod comment;
/// Mock implementations for member repository opers.
pub mod member;
/// Mock implementations for member invitation repository opers.
pub mod member_invitation;
mod nucl;
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
    pub comic_archives: Vec<ComicArchiveRecord>,
    pub prom_records: Vec<MockPromRecord>,
    pub deleted_image_keys: Vec<String>,
}

#[cfg_attr(test, derive(Clone))]
/// Immutable snapshot of the full mock state — used for asserting test outcomes.
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
    pub comic_archives: Vec<ComicArchiveRecord>,
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
            comic_archives: state.comic_archives,
            prom_records: state.prom_records,
            deleted_image_keys: state.deleted_image_keys,
        }
    }
}

/// The transactional context passed to [`Nucl::coord`] calls,
/// providing mutable access to the mock state during a simulated transaction.
pub struct MockContext {
    pub state: MockState,
    pub archive_commit_failure: bool,
    pub create_team_failure: bool,
}

#[cfg_attr(test, derive(Clone, Default))]
/// Toggleable failure flags for testing error paths in mock adapters.
pub struct MockFlags {
    pub token_failure: bool,
    pub image_get_failure: bool,
    pub image_put_failure: bool,

    pub image_head_failure: bool,
    pub image_head_absent: bool,

    pub image_delete_failure: bool,
    pub archive_commit_failure: bool,
    pub create_team_failure: bool,
}

/// The top-level mock repository and [`Nucl`] implementation.
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
        //
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
        //
        self.flags.lock().unwrap().token_failure = true;

        self
    }

    /// Enable image retrieval failures for subsequent opers.
    pub fn with_image_get_failure(self) -> Self {
        //
        self.flags.lock().unwrap().image_get_failure = true;

        self
    }

    /// Enable image storage failures for subsequent opers.
    pub fn with_image_put_failure(self) -> Self {
        //
        self.flags.lock().unwrap().image_put_failure = true;

        self
    }

    /// Enable head-object failures for subsequent opers.
    #[allow(dead_code)]
    pub fn with_image_head_failure(self) -> Self {
        //
        self.flags.lock().unwrap().image_head_failure = true;

        self
    }

    /// Report objects as absent for subsequent head-object opers.
    #[allow(dead_code)]
    pub fn with_image_head_absent(self) -> Self {
        //
        self.flags.lock().unwrap().image_head_absent = true;

        self
    }

    /// Enable delete-object failures for subsequent opers.
    #[allow(dead_code)]
    pub fn with_image_delete_failure(self) -> Self {
        //
        self.flags.lock().unwrap().image_delete_failure = true;

        self
    }

    /// Fail archive persistence before a transaction can commit.
    pub fn with_archive_commit_failure(self) -> Self {
        //
        self.flags.lock().unwrap().archive_commit_failure = true;

        self
    }

    /// Fail team creation before a transaction can commit.
    pub fn with_create_team_failure(self) -> Self {
        //
        self.flags.lock().unwrap().create_team_failure = true;

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

/// Build an expected-args [RootError] with a translated message.
pub fn expected(message: &str) -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl(message),
    }
}

/// Build an unrecoverable [RootError] with the given message.
pub fn unrecoverable(message: &str) -> BaseError {
    BaseError::Unrecoverable {
        message: message.into(),
    }
}

/// Return the current UTC timestamp.
pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

// run_reads_seeded_user(GetUserInfo)(positive): a seeded user should be readable outside a transaction.
// nucl_coord_commits_repo_and_prom(CreateMember, Defer)(positive): successful coordination should commit repo and prom state together.
// nucl_coord_rolls_back_repo_and_prom(CreateMember, Defer)(negative): failed coordination should discard repo and prom state together.

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
async fn run_reads_seeded_user() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1"),
        UserCredential {
            user_id: "user-1".into(),
            password_hash: "hash".into(),
        },
    );

    let found = mock.run(&GetUserInfo::Id { id: "user-1" }).await;

    assert!(found.is_ok());

    let found = found.ok().unwrap();

    assert_eq!(found.id, "user-1");
}

#[tokio::test]
async fn nucl_coord_commits_repo_and_prom() {
    //
    let mock = Mock::new();

    let member_entry = MemberEntry {
        id: "member-1".into(),
        user_id: "user-1".into(),
        user_nickname: "nick".into(),
        team_id: "team-1".into(),
        roles: RoleMask::from(RoleField::RAW_PROVIDER),
    };

    let repo = mock.clone();

    let prom = mock.clone();

    assert!(
        mock.coord(async move |context| {
            let create_member = CreateMember {
                entry: &member_entry,
            };

            repo.step(context, &create_member).await?;

            let prom_id = "prom-1".to_string();

            let payload = Payload::Image(image::Payload::CheckUpload {
                resource_kind: image::ResourceKind::UserAvatar,
                resource_id: "user-1".to_string(),
                object_key: "key".to_string(),
                version: 1,
            });

            let task = Task {
                id: &prom_id,
                payload: &payload,
                delay: None,
            };

            prom.step(context, &Defer::new(task)).await?;

            Ok::<(), BaseError>(())
        })
        .await
        .is_ok()
    );

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.members.len(), 1);

    assert_eq!(snapshot.prom_records.len(), 1);
}

#[tokio::test]
async fn nucl_coord_rolls_back_repo_and_prom() {
    //
    let mock = Mock::new();

    let member_entry = MemberEntry {
        id: "member-1".into(),
        user_id: "user-1".into(),
        user_nickname: "nick".into(),
        team_id: "team-1".into(),
        roles: RoleMask::from(RoleField::RAW_PROVIDER),
    };

    let repo = mock.clone();

    let prom = mock.clone();

    let err = mock
        .coord(async move |context| {
            //
            repo.step(
                context,
                &CreateMember {
                    entry: &member_entry,
                },
            )
            .await?;

            let prom_id = "prom-1".to_string();

            let payload = Payload::Image(image::Payload::Delete {
                object_key: "key".to_string(),
            });

            let task = Task {
                id: &prom_id,
                payload: &payload,
                delay: None,
            };

            prom.step(context, &Defer::new(task)).await?;

            Err::<(), _>(unrecoverable(
                "[nucl_coord_rolls_back_repo_and_prom] fail",
            ))
        })
        .await
        .err()
        .unwrap();

    assert!(matches!(
        err,
        NuclError::Step(BaseError::Unrecoverable { .. })
    ));

    let snapshot = mock.snapshot();

    assert!(snapshot.members.is_empty());

    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn nucl_coord_commits_state() {
    //
    let mock = Mock::new();

    Nucl::coord(&mock, async |context| {
        //
        context.state.users.push(user("user-1"));

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.users.len(), 1);
}

#[tokio::test]
async fn nucl_coord_rolls_back_state() {
    //
    let mock = Mock::new();

    let error = Nucl::coord(&mock, async |context| {
        //
        context.state.users.push(user("user-1"));

        Err::<(), _>(unrecoverable("[nucl_coord_rolls_back_state] fail"))
    })
    .await
    .unwrap_err();

    assert!(matches!(
        error,
        NuclError::Step(BaseError::Unrecoverable { .. })
    ));

    let snapshot = mock.snapshot();

    assert!(snapshot.users.is_empty());
}
