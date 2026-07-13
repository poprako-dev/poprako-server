//! Comment use cases — list and create team board comments.

use poprako_orchestra::{Nucl, run_proxy};

use crate::complex::comment::{CommentComplex, CommentPermComplex};
use crate::data::comment::CommentInfoVal;
use crate::data::comment::CreateCommentParams;
use crate::data::comment::CreateCommentPayload;
use crate::data::comment::ListCommentInfosParams;
use crate::model::comment::CommentEntry;
use crate::model::comment::CommentListSpec;
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::repo::comment::CommentRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::comment::{CreateComment, ListCommentInfos};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::result::{RegularError, RegularResult};

#[cfg(test)]
mod tests;

/// Lists comments under a team.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    params: ListCommentInfosParams,
) -> RegularResult<Vec<CommentInfoVal>>
where
    R: CommentRepo<C> + MemberRepo<C> + Sync,
    I: ImagePool,
{
    let comment_list_spec: CommentListSpec = params.into();

    CommentPermComplex::can_user_list_infos(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &comment_list_spec.team_id,
    )
    .await?;

    let comment_infos = repo
        .run(&ListCommentInfos {
            spec: &comment_list_spec,
        })
        .await?;

    let mut comment_info_vals = Vec::with_capacity(comment_infos.len());

    for comment_info in comment_infos {
        comment_info_vals
            .push(CommentInfoVal::from_model(image_pool, comment_info).await?);
    }

    Ok(comment_info_vals)
}

/// Creates a comment under a team.
pub async fn create<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: CreateCommentParams,
) -> RegularResult<CreateCommentPayload>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: CommentRepo<C> + MemberRepo<C> + Send + Sync,
{
    CommentPermComplex::can_user_create(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &params.team_id,
    )
    .await?;

    let comment_info = nucl
        .coord(async move |context| {
            let comment_entry = CommentEntry {
                id: CommentComplex::gen_id(),
                team_id: params.team_id,
                user_id: token.user_id,
                content: params.content,
            };

            repo.step(
                context,
                &CreateComment {
                    entry: &comment_entry,
                },
            )
            .await
        })
        .await?;

    Ok(CreateCommentPayload {
        id: comment_info.id,
    })
}
