//! Mock comment repository operations.

use std::cmp::Reverse;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::comment::{CommentEntry, CommentInfo, CommentListSpec};
use crate::model::user::UserInfo;
use crate::part::repo::comment::CommentRepo;
use crate::part::repo::oper::comment::{CreateComment, ListCommentInfos};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{BaseError, BaseResult, accept};
use crate::value::comment::CommentInclOpt;

impl CommentRepo<MockContext> for Mock {}

fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

fn apply_user_incl(
    state: &MockState,
    comment_info: &mut CommentInfo,
    include_user: bool,
) {
    //
    comment_info.user = None;

    if include_user {
        comment_info.user = find_user(state, &comment_info.user_id);
    }
}

fn list_comments(
    state: &MockState,
    spec: &CommentListSpec,
) -> Vec<CommentInfo> {
    //
    let include_user = spec.incl_opt.contains(&CommentInclOpt::User);

    let mut comment_infos = state
        .comments
        .iter()
        .filter(|comment_info| comment_info.team_id == spec.team_id)
        .cloned()
        .collect::<Vec<_>>();

    comment_infos.sort_by_key(|comment_info| Reverse(comment_info.created_at));

    for comment_info in &mut comment_infos {
        apply_user_incl(state, comment_info, include_user);
    }

    let offset = spec.offset as usize;

    let limit = spec.limit as usize;

    match offset >= comment_infos.len() {
        //
        true => Vec::new(),

        false => {
            //
            let end = std::cmp::min(offset + limit, comment_infos.len());

            comment_infos[offset..end].to_vec()
        }
    }
}

fn create_comment(
    state: &mut MockState,
    entry: &CommentEntry,
) -> BaseResult<CommentInfo> {
    //
    if state
        .comments
        .iter()
        .any(|comment_info| comment_info.id == entry.id)
    {
        return Err(expected("error-already-exists"));
    }

    let comment_info = CommentInfo {
        id: entry.id.clone(),
        team_id: entry.team_id.clone(),
        user_id: entry.user_id.clone(),
        user: None,
        content: entry.content.clone(),
        created_at: now(),
    };

    state.comments.push(comment_info.clone());

    accept(comment_info)
}

impl Run<ListCommentInfos<'_>> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListCommentInfos<'_>,
    ) -> BaseResult<Vec<CommentInfo>> {
        //
        let state = self.state.lock().unwrap();

        accept(list_comments(&state, oper.spec))
    }
}

impl Step<CreateComment<'_>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateComment<'_>,
    ) -> BaseResult<CommentInfo> {
        create_comment(&mut context.state, oper.entry)
    }
}
