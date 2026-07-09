//! Comment use cases — list and create team board comments.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::complex::comment::{CommentComplex, CommentPermComplex};
use crate::data::comment::{
    CommentInfoVal, CreateCommentData, CreateCommentVal, ListCommentInfosData,
};
use crate::model::comment::{CommentForm, CommentListSpec};
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::repo::comment::{CommentRepo, CommentRepoTransactional};
use crate::part::repo::map_drive_err;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::step::comment::CommentStep;
use crate::result::{RegularError, RegularResult, accept};
use crate::util::DeriveTransactional;

#[cfg(test)]
mod tests;

/// Lists comments under a team.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    data: ListCommentInfosData,
) -> RegularResult<Vec<CommentInfoVal>>
where
    R: CommentRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional:
        CommentRepoTransactional<C> + MemberRepoTransactional<C>,
    I: ImagePool,
{
    let comment_list_spec: CommentListSpec = data.into();

    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    CommentPermComplex::can_user_list_infos(
        &mut repo.as_proxy(),
        &token.user_id,
        &comment_list_spec.team_id,
    )
    .await?;

    let comment_infos = repo
        .execute(&CommentStep::list_infos(&comment_list_spec))
        .await?;

    let mut comment_info_vals = Vec::with_capacity(comment_infos.len());

    for comment_info in comment_infos {
        comment_info_vals
            .push(CommentInfoVal::from_model(image_pool, comment_info).await?);
    }

    accept(comment_info_vals)
}

/// Creates a comment under a team.
pub async fn create<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: CreateCommentData,
) -> RegularResult<CreateCommentVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: CommentRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        CommentRepoTransactional<C> + MemberRepoTransactional<C> + Send + Sync,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    CommentPermComplex::can_user_create(
        &mut repo.as_proxy(),
        &token.user_id,
        &data.team_id,
    )
    .await?;

    let comment_info = drive
        .with_context(async move |context| {
            //
            let repo = repo.derive_transactional().await;

            let comment_form = CommentForm {
                id: CommentComplex::gen_id(),
                team_id: data.team_id,
                user_id: token.user_id,
                content: data.content,
            };

            let comment_info = repo
                .advance(context, &CommentStep::create(&comment_form))
                .await?;

            accept(comment_info)
        })
        .await
        .map_err(map_drive_err)?;

    accept(CreateCommentVal {
        id: comment_info.id,
    })
}
