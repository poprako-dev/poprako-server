//! Mock fixture construction.

use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::model::meta::ObjMeta;

use crate::model::read::proj::announcement::AnnouncementInfo;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::assignment_invitation::AssignmentInvitationInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::comment::CommentInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::member_invitation::MemberInvitationInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::system_mail::SystemMailInfo;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::term::TermInfo;
use crate::model::read::proj::termbase::TermbaseInfo;
use crate::model::read::proj::unit::UnitInfo;
use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::read::proj::workset::WorksetInfo;
use crate::part_impl::repo::mock_impl::{Mock, MockObjRecord};

impl Mock {
    /// Seed a user and its credential directly into the mock state.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    pub fn seed_user(&self, user: UserInfo, credential: UserCredential) {
        //
        let mut state = self.state.lock().unwrap();

        state.users.push(user);

        state.credentials.push(credential);
    }

    /// Seed an announcement directly into the mock state.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    pub fn seed_announcement(&self, announcement: AnnouncementInfo) {
        self.state.lock().unwrap().announcements.push(announcement);
    }

    /// Seed a comment directly into the mock state.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    pub fn seed_comment(&self, comment: CommentInfo) {
        self.state.lock().unwrap().comments.push(comment);
    }

    /// Seed a team directly into the mock state.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    pub fn seed_team(&self, team: TeamInfo) {
        self.state.lock().unwrap().teams.push(team);
    }

    /// Seed a member directly into the mock state.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    pub fn seed_member(&self, member: MemberInfo) {
        self.state.lock().unwrap().members.push(member);
    }

    /// Seed a member invitation directly into the mock state.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    pub fn seed_member_invitation(
        &self,
        member_invitation: MemberInvitationInfo,
    ) {
        //
        self.state
            .lock()
            .unwrap()
            .member_invitations
            .push(member_invitation);
    }

    /// Seed a workset directly into the mock state.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    pub fn seed_workset(&self, workset: WorksetInfo) {
        self.state.lock().unwrap().worksets.push(workset);
    }

    /// Seed a comic directly into the mock state.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    pub fn seed_comic(&self, comic: ComicInfo) {
        self.state.lock().unwrap().comics.push(comic);
    }

    /// Seed a terminology base directly into the mock state.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    pub fn seed_termbase(&self, termbase: TermbaseInfo) {
        self.state.lock().unwrap().termbases.push(termbase);
    }

    /// Seed a terminology entry directly into the mock state.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    pub fn seed_term(&self, term: TermInfo) {
        self.state.lock().unwrap().terms.push(term);
    }

    /// Seed a chapter directly into the mock state.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    pub fn seed_chapter(&self, chapter: ChapterInfo) {
        self.state.lock().unwrap().chapters.push(chapter);
    }

    /// Seed an assignment directly into the mock state.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    pub fn seed_assignment(&self, assignment: AssignmentInfo) {
        self.state.lock().unwrap().assignments.push(assignment);
    }

    /// Seed an assignment invitation directly into the mock state.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    pub fn seed_assignment_invitation(
        &self,
        assignment_invitation: AssignmentInvitationInfo,
    ) {
        //
        self.state
            .lock()
            .unwrap()
            .assignment_invitations
            .push(assignment_invitation);
    }

    /// Seed a page directly into the mock state.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    pub fn seed_page(&self, page: PageInfo) {
        self.state.lock().unwrap().pages.push(page);
    }

    /// Seed one verified page-image object for read-projection tests.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    #[expect(
        clippy::uninlined_format_args,
        reason = "Repository formatting keeps interpolation arguments explicit"
    )]
    pub fn seed_page_image_obj(&self, id: &str, ext: &str) {
        //
        let meta = ObjMeta {
            key: ObjKey {
                id: id.to_owned(),
                ver: 1,
                image: format!("page/chapter_test/{}-1.{}", id, ext),
            },
            is_avail: true,
            hash: vec![0; 32],
            ext: ext.to_owned(),
        };

        self.state
            .lock()
            .unwrap()
            .objs
            .entry("page_image")
            .or_default()
            .insert(
                id.to_owned(),
                MockObjRecord {
                    version: 1,
                    meta: Some(meta),
                },
            );
    }

    /// Seed a unit directly into the mock state.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    pub fn seed_unit(&self, unit: UnitInfo) {
        self.state.lock().unwrap().units.push(unit);
    }

    /// Seed a system mail directly into the mock state.
    /// # Panics
    /// Panics if the mock state, flags, or event mutex is poisoned.
    pub fn seed_system_mail(&self, system_mail: SystemMailInfo) {
        self.state.lock().unwrap().system_mails.push(system_mail);
    }
}
