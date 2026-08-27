//! Mock comment repository operations.

use std::cmp::Reverse;

use poprako_orchestra::Run;
use tracing::instrument;

use crate::model::read::proj::comment::CommentInfo;
use crate::model::read::proj::user::UserInfo;
use crate::model::read::spec::comment::CommentListSpec;
use crate::model::write::comment::CommentEntry;
use crate::part::repo::oper::comment::{CreateComment, ListCommentInfos};
use crate::part_impl::repo::mock_impl::{Mock, MockState, expected, now};
use crate::result::{BaseError, BaseRest, accept};
use crate::value::comment::CommentInclOpt;

// Internal implementation of `find_user`.
fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    //
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

// Internal implementation of `apply_user_incl`.
fn apply_user_incl(
    state: &MockState,
    comment_info: &mut CommentInfo,
    include_user: bool,
) {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    comment_info.user = None;

    if include_user {
        comment_info.user = find_user(state, &comment_info.user_id);
    }
}

// Internal implementation of `list_comments`.
fn list_comments(
    state: &MockState,
    spec: &CommentListSpec,
) -> Vec<CommentInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
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

    if offset >= comment_infos.len() {
        Vec::new()
    } else {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let end = std::cmp::min(offset + limit, comment_infos.len());

        comment_infos[offset..end].to_vec()
    }
}

// Internal implementation of `create_comment`.
fn create_comment(
    state: &mut MockState,
    entry: &CommentEntry,
) -> BaseRest<CommentInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
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
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListCommentInfos<'_>,
    ) -> BaseRest<Vec<CommentInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(list_comments(&state, oper.spec))
    }
}

impl Run<CreateComment<'_>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &CreateComment<'_>) -> BaseRest<CommentInfo> {
        //
        let mut state = self.state.lock().unwrap();

        create_comment(&mut state, oper.entry)
    }
}
