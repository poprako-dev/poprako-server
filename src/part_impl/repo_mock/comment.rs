//! Mock implementations of `CommentRepo` and `CommentRepoTransactional`.

use async_trait::async_trait;

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_transactional::drive::result::Error as DriveError;

use crate::model::comment::{CommentForm, CommentInfo, CommentListSpec};
use crate::model::user::UserInfo;
use crate::part::repo::comment::{CommentRepo, CommentRepoTransactional};
use crate::part::repo::step::comment::{Create, ListInfos};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_mock::{Mock, MockContext, MockState, MockTransactional, expected, now};
use crate::result::RootError;
use crate::util::DeriveTransactional;
use crate::value::comment::CommentInclOpt;

impl CommentRepo<MockContext> for Mock {}

impl CommentRepoTransactional<MockContext> for MockTransactional {}

fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

fn apply_user_incl(state: &MockState, comment_info: &mut CommentInfo, include_user: bool) {
    comment_info.user = None;

    if include_user {
        comment_info.user = find_user(state, &comment_info.user_id);
    }
}

fn list_comments(state: &MockState, spec: &CommentListSpec) -> Vec<CommentInfo> {
    let include_user = spec.incl_opt.contains(&CommentInclOpt::User);
    let mut comment_infos = state
        .comments
        .iter()
        .filter(|comment_info| comment_info.team_id == spec.team_id)
        .cloned()
        .collect::<Vec<_>>();

    comment_infos.sort_by(|left, right| right.created_at.cmp(&left.created_at));

    for comment_info in &mut comment_infos {
        apply_user_incl(state, comment_info, include_user);
    }

    let offset = spec.offset as usize;
    let limit = spec.limit as usize;

    if offset >= comment_infos.len() {
        return Vec::new();
    }

    let end = std::cmp::min(offset + limit, comment_infos.len());
    comment_infos[offset..end].to_vec()
}

fn create_comment(state: &mut MockState, form: &CommentForm) -> Result<CommentInfo, RootError> {
    if state
        .comments
        .iter()
        .any(|comment_info| comment_info.id == form.id)
    {
        return Err(expected("error-already-exists"));
    }

    let comment_info = CommentInfo {
        id: form.id.clone(),
        team_id: form.team_id.clone(),
        user_id: form.user_id.clone(),
        user: None,
        content: form.content.clone(),
        created_at: now(),
    };

    state.comments.push(comment_info.clone());

    Ok(comment_info)
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &ListInfos<'a>) -> Result<Vec<CommentInfo>, Self::Error> {
        let state = self.state.lock().unwrap();

        Ok(list_comments(&state, step.spec))
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Create<'a>,
    ) -> Result<CommentInfo, Self::Error> {
        create_comment(&mut context.state, step.form)
    }
}

// list_infos_filters_sorts_pages_and_includes_user(ListInfos)(positive): list should filter team, sort by created_at desc, page, and honor User include.
// list_infos_omits_user_without_include(ListInfos)(positive): list should clear user data when User include is absent.
// create_persists_comment(Create)(positive): create should append one comment.
// create_rejects_duplicate_id(Create)(negative): duplicate id should return an argument error.

use time::OffsetDateTime;

use crate::model::user::UserCredential;
use crate::part::repo::step::comment::CommentStep;
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

fn comment(id: &str, team_id: &str, user_id: &str, created_at: OffsetDateTime) -> CommentInfo {
    CommentInfo {
        id: id.into(),
        team_id: team_id.into(),
        user_id: user_id.into(),
        user: Some(user(user_id)),
        content: "content".into(),
        created_at,
    }
}

fn form(id: &str) -> CommentForm {
    CommentForm {
        id: id.into(),
        team_id: "team-1".into(),
        user_id: "user-1".into(),
        content: "content".into(),
    }
}

fn spec(incl_opt: Vec<CommentInclOpt>, offset: u64, limit: u64) -> CommentListSpec {
    CommentListSpec {
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
    mock.seed_comment(comment("comment-old", "team-1", "user-1", time));
    mock.seed_comment(comment(
        "comment-new",
        "team-1",
        "user-1",
        time + time::Duration::seconds(10),
    ));
    mock.seed_comment(comment(
        "comment-other-team",
        "team-2",
        "user-1",
        time + time::Duration::seconds(20),
    ));

    let comment_infos = mock
        .execute(&CommentStep::list_infos(&spec(
            vec![CommentInclOpt::User],
            0,
            1,
        )))
        .await
        .ok()
        .unwrap();

    assert_eq!(comment_infos.len(), 1);
    assert_eq!(comment_infos[0].id, "comment-new");
    assert_eq!(comment_infos[0].user.as_ref().unwrap().id, "user-1");
}

#[tokio::test]
async fn list_infos_omits_user_without_include() {
    let mock = Mock::new();
    mock.seed_user(user("user-1"), credential("user-1"));
    mock.seed_comment(comment("comment-1", "team-1", "user-1", now()));

    let comment_infos = mock
        .execute(&CommentStep::list_infos(&spec(Vec::new(), 0, 10)))
        .await
        .ok()
        .unwrap();

    assert!(comment_infos[0].user.is_none());
}

#[tokio::test]
async fn create_persists_comment() {
    let mock = Mock::new();
    let comment_form = form("comment-1");
    let repo = mock.transactional().await;

    assert!(
        mock.with_context(async move |context| {
            repo.advance(context, &CommentStep::create(&comment_form))
                .await
        })
        .await
        .is_ok()
    );
    assert_eq!(mock.snapshot().comments.len(), 1);
}

#[tokio::test]
async fn create_rejects_duplicate_id() {
    let mock = Mock::new();
    mock.seed_comment(comment("comment-1", "team-1", "user-1", now()));
    let comment_form = form("comment-1");
    let repo = mock.transactional().await;

    let err = mock
        .with_context(async move |context| {
            repo.advance(context, &CommentStep::create(&comment_form))
                .await
        })
        .await
        .err()
        .unwrap();

    let DriveError::Advance(err) = err else {
        panic!("expected advance error");
    };

    assert_expected_variant(err, ExpectedVariant::ArgsInvalid);
    assert_eq!(mock.snapshot().comments.len(), 1);
}
