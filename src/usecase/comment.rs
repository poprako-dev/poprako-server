//! Comment use cases — list and create team board comments.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::complex::comment::{CommentComplex, CommentPermComplex};
use crate::data::comment_data;
use crate::model::comment_model;
use crate::model::user_model;
use crate::part::image::ImagePool;
use crate::part::repo::comment::{CommentRepo, CommentRepoTransactional};
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::step::comment::CommentStep;
use crate::result::{RegularError, RegularResult};
use crate::util::DeriveTransactional;

#[cfg(test)]
mod tests;

/// Lists comments under a team.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: user_model::Token,
    data: comment_data::ListInfosData,
) -> RegularResult<Vec<comment_data::InfoVal>>
where
    R: CommentRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional:
        CommentRepoTransactional<C> + MemberRepoTransactional<C>,
    I: ImagePool,
{
    let comment_list_spec: comment_model::ListSpec = data.into();

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
        comment_info_vals.push(
            comment_data::InfoVal::from_model(image_pool, comment_info).await?,
        );
    }

    Ok(comment_info_vals)
}

/// Creates a comment under a team.
pub async fn create<D, C, R>(
    drive: &D,
    repo: &R,
    token: user_model::Token,
    data: comment_data::CreateData,
) -> RegularResult<comment_data::CreateVal>
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
        .with_context(
            async move |context| -> RegularResult<comment_model::Info> {
                //
                let repo = repo.derive_transactional().await;

                let comment_form = comment_model::Form {
                    id: CommentComplex::gen_id(),
                    team_id: data.team_id,
                    user_id: token.user_id,
                    content: data.content,
                };

                let comment_info = repo
                    .advance(context, &CommentStep::create(&comment_form))
                    .await?;

                Ok(comment_info)
            },
        )
        .await?;

    Ok(comment_data::CreateVal {
        id: comment_info.id,
    })
}
