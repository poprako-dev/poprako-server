//! Comment use cases — list and create team board comments.

use poprako_orchestra::{Nucl, OperRun as _, OperStep as _, run_proxy};
use tracing::instrument;

use crate::complex::comment::{CommentComplex, CommentPermComplex};
use crate::data::instr::comment::{CreateCommentInstr, ListCommentInfosInstr};
use crate::data::val::comment::CreateCommentVal;
use crate::data::view::comment::CommentInfoView;
use crate::model::read::spec::comment::CommentListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::comment::CommentEntry;
use crate::part::image::ImagePool;
use crate::part::repo::comment::CommentRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::comment::{CreateComment, ListCommentInfos};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::result::{BaseError, BaseRest, accept};

#[cfg(test)]
// Unit tests that validate comment lifecycle and visibility constraints.
mod tests;

/// Lists comments under a team.
#[instrument(level = "info", err(Debug), skip(repo, image_pool))]
pub async fn list_infos<C, R, I>(
    (repo, image_pool): (&R, &I),
    token: UserToken,
    instr: ListCommentInfosInstr,
) -> BaseRest<Vec<CommentInfoView>>
where
    R: CommentRepo<C> + MemberRepo<C> + Sync,
    I: ImagePool,
{
    let comment_list_spec: CommentListSpec = instr.into();

    CommentPermComplex::ensure_user_can_list_infos(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &comment_list_spec.team_id,
    )
    .await?;

    let comment_infos = ListCommentInfos {
        spec: &comment_list_spec,
    }
    .run_on(repo)
    .await?;

    let mut comment_info_vals = Vec::with_capacity(comment_infos.len());

    for comment_info in comment_infos {
        comment_info_vals
            .push(CommentInfoView::from_model(image_pool, comment_info).await?);
    }

    accept(comment_info_vals)
}

/// Creates a comment under a team.
#[instrument(level = "info", err(Debug), skip(nucl, repo))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: CreateCommentInstr,
) -> BaseRest<CreateCommentVal>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: CommentRepo<C> + MemberRepo<C> + Send + Sync,
{
    CommentPermComplex::ensure_user_can_create(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &instr.team_id,
    )
    .await?;

    let comment_info = nucl
        .coord(async move |context| {
            //
            let comment_entry = CommentEntry {
                id: CommentComplex::gen_id(),
                team_id: instr.team_id,
                user_id: token.user_id,
                content: instr.content,
            };

            CreateComment {
                entry: &comment_entry,
            }
            .step_on(repo, context)
            .await
        })
        .await?;

    accept(CreateCommentVal {
        id: comment_info.id,
    })
}
