//! Mock implementations of `AnnouncementRepo` and `AnnouncementRepoTransactional`.

use async_trait::async_trait;

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_transactional::drive::result::Error as DriveError;

use crate::model::announcement::{AnnouncementForm, AnnouncementInfo, AnnouncementListSpec};
use crate::model::user::UserInfo;
use crate::part::repo::announcement::{AnnouncementRepo, AnnouncementRepoTransactional};
use crate::part::repo::step::announcement::{Create, ListInfos};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_mock::{Mock, MockContext, MockState, MockTransactional, expected, now};
use crate::result::{RegularError, RegularResult};
use crate::util::DeriveTransactional;
use crate::value::announcement::AnnouncementInclOpt;

impl AnnouncementRepo<MockContext> for Mock {}

impl AnnouncementRepoTransactional<MockContext> for MockTransactional {}

fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

fn apply_user_incl(
    state: &MockState,
    announcement_info: &mut AnnouncementInfo,
    include_user: bool,
) {
    announcement_info.user = None;

    if include_user {
        announcement_info.user = find_user(state, &announcement_info.user_id);
    }
}

fn list_announcements(state: &MockState, spec: &AnnouncementListSpec) -> Vec<AnnouncementInfo> {
    let include_user = spec.incl_opt.contains(&AnnouncementInclOpt::User);
    let mut announcement_infos = state
        .announcements
        .iter()
        .filter(|announcement_info| announcement_info.team_id == spec.team_id)
        .cloned()
        .collect::<Vec<_>>();

    announcement_infos.sort_by(|left, right| right.created_at.cmp(&left.created_at));

    for announcement_info in &mut announcement_infos {
        apply_user_incl(state, announcement_info, include_user);
    }

    let offset = spec.offset as usize;
    let limit = spec.limit as usize;

    if offset >= announcement_infos.len() {
        return Vec::new();
    }

    let end = std::cmp::min(offset + limit, announcement_infos.len());
    announcement_infos[offset..end].to_vec()
}

fn create_announcement(
    state: &mut MockState,
    form: &AnnouncementForm,
) -> RegularResult<AnnouncementInfo> {
    if state
        .announcements
        .iter()
        .any(|announcement_info| announcement_info.id == form.id)
    {
        return Err(expected("error-already-exists"));
    }

    let announcement_info = AnnouncementInfo {
        id: form.id.clone(),
        team_id: form.team_id.clone(),
        user_id: form.user_id.clone(),
        user: None,
        title: form.title.clone(),
        content: form.content.clone(),
        created_at: now(),
    };

    state.announcements.push(announcement_info.clone());

    Ok(announcement_info)
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for Mock {
    type Error = RegularError;

    async fn execute(&self, step: &ListInfos<'a>) -> Result<Vec<AnnouncementInfo>, Self::Error> {
        let state = self.state.lock().unwrap();

        Ok(list_announcements(&state, step.spec))
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Create<'a>,
    ) -> Result<AnnouncementInfo, Self::Error> {
        create_announcement(&mut context.state, step.form)
    }
}

// list_infos_filters_sorts_pages_and_includes_user(ListInfos)(positive): list should filter team, sort by created_at desc, page, and honor User include.
// list_infos_omits_user_without_include(ListInfos)(positive): list should clear user data when User include is absent.
// create_persists_announcement(Create)(positive): create should append one announcement.
// create_rejects_duplicate_id(Create)(negative): duplicate id should return an argument error.

use time::OffsetDateTime;

use crate::model::user::UserCredential;
use crate::part::repo::step::announcement::AnnouncementStep;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;

fn user(id: &str) -> UserInfo {
    let time = now();

    UserInfo {
        id: id.into(),
        qid: id.into(),
        nickname: id.into(),
        avatar_key: None,
        avatar_uploaded: false,
        avatar_version: 0,
        is_sadmin: false,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn credential(user_id: &str) -> UserCredential {
    UserCredential {
        user_id: user_id.into(),
        password_hash: "hash".into(),
    }
}

fn announcement(
    id: &str,
    team_id: &str,
    user_id: &str,
    created_at: OffsetDateTime,
) -> AnnouncementInfo {
    AnnouncementInfo {
        id: id.into(),
        team_id: team_id.into(),
        user_id: user_id.into(),
        user: Some(user(user_id)),
        title: "title".into(),
        content: "content".into(),
        created_at,
    }
}

fn form(id: &str) -> AnnouncementForm {
    AnnouncementForm {
        id: id.into(),
        team_id: "team-1".into(),
        user_id: "user-1".into(),
        title: "title".into(),
        content: "content".into(),
    }
}

fn spec(incl_opt: Vec<AnnouncementInclOpt>, offset: u64, limit: u64) -> AnnouncementListSpec {
    AnnouncementListSpec {
        team_id: "team-1".into(),
        incl_opt,
        offset,
        limit,
    }
}

#[tokio::test]
async fn list_infos_filters_sorts_pages_and_includes_user() {
    let mock = Mock::new();
    let time = now();
    mock.seed_user(user("user-1"), credential("user-1"));
    mock.seed_announcement(announcement("announcement-old", "team-1", "user-1", time));
    mock.seed_announcement(announcement(
        "announcement-new",
        "team-1",
        "user-1",
        time + time::Duration::seconds(10),
    ));
    mock.seed_announcement(announcement(
        "announcement-other-team",
        "team-2",
        "user-1",
        time + time::Duration::seconds(20),
    ));

    let announcement_infos = mock
        .execute(&AnnouncementStep::list_infos(&spec(
            vec![AnnouncementInclOpt::User],
            0,
            1,
        )))
        .await
        .ok()
        .unwrap();

    assert_eq!(announcement_infos.len(), 1);
    assert_eq!(announcement_infos[0].id, "announcement-new");
    assert_eq!(announcement_infos[0].user.as_ref().unwrap().id, "user-1");
}

#[tokio::test]
async fn list_infos_omits_user_without_include() {
    let mock = Mock::new();
    mock.seed_user(user("user-1"), credential("user-1"));
    mock.seed_announcement(announcement("announcement-1", "team-1", "user-1", now()));

    let announcement_infos = mock
        .execute(&AnnouncementStep::list_infos(&spec(Vec::new(), 0, 10)))
        .await
        .ok()
        .unwrap();

    assert!(announcement_infos[0].user.is_none());
}

#[tokio::test]
async fn create_persists_announcement() {
    let mock = Mock::new();
    let announcement_form = form("announcement-1");
    let repo = mock.derive_transactional().await;

    assert!(
        mock.with_context(async move |context| {
            repo.advance(context, &AnnouncementStep::create(&announcement_form))
                .await
        })
        .await
        .is_ok()
    );
    assert_eq!(mock.snapshot().announcements.len(), 1);
}

#[tokio::test]
async fn create_rejects_duplicate_id() {
    let mock = Mock::new();
    mock.seed_announcement(announcement("announcement-1", "team-1", "user-1", now()));
    let announcement_form = form("announcement-1");
    let repo = mock.derive_transactional().await;

    let err = mock
        .with_context(async move |context| {
            repo.advance(context, &AnnouncementStep::create(&announcement_form))
                .await
        })
        .await
        .err()
        .unwrap();

    let DriveError::Advance(err) = err else {
        panic!("expected advance error");
    };

    assert_expected_variant(err, ExpectedVariant::Args);
    assert_eq!(mock.snapshot().announcements.len(), 1);
}
