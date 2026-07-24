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
use crate::model::term::TermInfo;
use crate::model::termbase::TermbaseInfo;
use crate::model::unit::UnitInfo;
use crate::model::user::{UserCredential, UserInfo};
use crate::model::workset::WorksetInfo;
use crate::part::effect::event::Event;
use crate::part::prom::payload::{TaskPayload, image};
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
/// Mock implementations for term repository opers.
pub mod term;
/// Mock implementations for termbase repository opers.
pub mod termbase;
#[cfg(test)]
mod tests;
/// Mock implementations for unit repository opers.
pub mod unit;
/// Mock implementations for user repository opers.
pub mod user;
/// Mock implementations for workset repository opers.
pub mod workset;

/// In-memory state holding all mock repository records.
#[cfg_attr(test, derive(Clone, Default))]
pub struct MockState {
    /// Mock storage for user records.
    pub users: Vec<UserInfo>,
    /// Mock storage for user credential records.
    pub credentials: Vec<UserCredential>,
    /// Mock storage for announcement records.
    pub announcements: Vec<AnnouncementInfo>,
    /// Mock storage for comment records.
    pub comments: Vec<CommentInfo>,
    /// Mock storage for team records.
    pub teams: Vec<TeamInfo>,
    /// Mock storage for team membership records.
    pub members: Vec<MemberInfo>,
    /// Mock storage for pending member invitation records.
    pub member_invitations: Vec<MemberInvitationInfo>,
    /// Mock storage for workset records.
    pub worksets: Vec<WorksetInfo>,
    /// Mock storage for comic records.
    pub comics: Vec<ComicInfo>,
    /// Mock storage for termbase records.
    pub termbases: Vec<TermbaseInfo>,
    /// Mock storage for terminology entry records.
    pub terms: Vec<TermInfo>,
    /// Mock storage for chapter records.
    pub chapters: Vec<ChapterInfo>,
    /// Mock storage for assignment records.
    pub assignments: Vec<AssignmentInfo>,
    /// Mock storage for assignment invitation records.
    pub assignment_invitations: Vec<AssignmentInvitationInfo>,
    /// Mock storage for page records.
    pub pages: Vec<PageInfo>,
    /// Mock storage for unit records.
    pub units: Vec<UnitInfo>,
    /// Mock storage for system mail records.
    pub system_mails: Vec<SystemMailInfo>,
    /// Mock storage for archived comic records.
    pub comic_archives: Vec<ComicArchiveRecord>,
    /// Mock storage for deferred prom action records.
    pub prom_records: Vec<MockPromRecord>,
    /// Mock storage for keys of images deleted from object storage.
    pub deleted_image_keys: Vec<String>,
}

#[cfg_attr(test, derive(Clone))]
/// Immutable snapshot of the full mock state — used for asserting test outcomes.
pub struct MockSnapshot {
    /// Snapshot of user records at the capture time.
    pub users: Vec<UserInfo>,
    /// Snapshot of credential records at the capture time.
    pub credentials: Vec<UserCredential>,
    /// Snapshot of announcement records at the capture time.
    pub announcements: Vec<AnnouncementInfo>,
    /// Snapshot of comment records at the capture time.
    pub comments: Vec<CommentInfo>,
    /// Snapshot of team records at the capture time.
    pub teams: Vec<TeamInfo>,
    /// Snapshot of membership records at the capture time.
    pub members: Vec<MemberInfo>,
    /// Snapshot of member invitation records at the capture time.
    pub member_invitations: Vec<MemberInvitationInfo>,
    /// Snapshot of workset records at the capture time.
    pub worksets: Vec<WorksetInfo>,
    /// Snapshot of comic records at the capture time.
    pub comics: Vec<ComicInfo>,
    /// Snapshot of termbase records at the capture time.
    pub termbases: Vec<TermbaseInfo>,
    /// Snapshot of terminology entry records at the capture time.
    pub terms: Vec<TermInfo>,
    /// Snapshot of chapter records at the capture time.
    pub chapters: Vec<ChapterInfo>,
    /// Snapshot of assignment records at the capture time.
    pub assignments: Vec<AssignmentInfo>,
    /// Snapshot of assignment invitation records at the capture time.
    pub assignment_invitations: Vec<AssignmentInvitationInfo>,
    /// Snapshot of page records at the capture time.
    pub pages: Vec<PageInfo>,
    /// Snapshot of unit records at the capture time.
    pub units: Vec<UnitInfo>,
    /// Snapshot of system mail records at the capture time.
    pub system_mails: Vec<SystemMailInfo>,
    /// Snapshot of archived comic records at the capture time.
    pub comic_archives: Vec<ComicArchiveRecord>,
    /// Snapshot of deferred prom action records at the capture time.
    pub prom_records: Vec<MockPromRecord>,
    /// Snapshot of deleted image keys at the capture time.
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
            termbases: state.termbases,
            terms: state.terms,
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
    /// Mutable mock repository state visible within the current transaction.
    pub state: MockState,
    /// When true, archive persistence will fail before transaction commit.
    pub archive_commit_failure: bool,
    /// When true, team creation will fail before transaction commit.
    pub create_team_failure: bool,
}

#[cfg_attr(test, derive(Clone, Default))]
/// Toggleable failure flags for testing error paths in mock adapters.
pub struct MockFlags {
    /// Simulates a token authentication failure.
    pub token_failure: bool,
    /// Simulates an image retrieval failure from object storage.
    pub image_get_failure: bool,
    /// Simulates an image upload failure to object storage.
    pub image_put_failure: bool,

    /// Simulates a failure in head-object metadata retrieval.
    pub image_head_failure: bool,
    /// Simulates the head-object reporting the object as absent.
    pub image_head_absent: bool,
    /// Simulates a SHA-256 hash mismatch in head-object response.
    pub image_head_hash_mismatch: bool,
    /// Simulates a content-length mismatch in head-object response.
    pub image_head_length_mismatch: bool,

    /// Simulates a failure in object deletion from storage.
    pub image_delete_failure: bool,
    /// Simulates a failure in archive persistence within a transaction.
    pub archive_commit_failure: bool,
    /// Simulates a failure in team creation within a transaction.
    pub create_team_failure: bool,
}

/// The top-level mock repository and [`Nucl`] implementation.
/// Wraps shared mutable state, failure flags, and an event buffer behind
/// `Arc<Mutex<...>>` for concurrent test access.
#[cfg_attr(test, derive(Clone, Default))]
pub struct Mock {
    /// Shared mutable mock repository state for concurrent test access.
    pub state: Arc<Mutex<MockState>>,
    /// Shared mutable mock failure flags for testing error paths.
    pub flags: Arc<Mutex<MockFlags>>,
    /// Shared event buffer collecting emitted domain events during tests.
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

    /// Seed a terminology base directly into the mock state.
    pub fn seed_termbase(&self, termbase: TermbaseInfo) {
        self.state.lock().unwrap().termbases.push(termbase);
    }

    /// Seed a terminology entry directly into the mock state.
    pub fn seed_term(&self, term: TermInfo) {
        self.state.lock().unwrap().terms.push(term);
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

    /// Report a mismatching SHA-256 checksum from head-object opers.
    #[allow(dead_code)]
    pub fn with_image_head_hash_mismatch(self) -> Self {
        //
        self.flags.lock().unwrap().image_head_hash_mismatch = true;

        self
    }

    /// Report a mismatching content length from head-object opers.
    #[allow(dead_code)]
    pub fn with_image_head_length_mismatch(self) -> Self {
        //
        self.flags.lock().unwrap().image_head_length_mismatch = true;

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
