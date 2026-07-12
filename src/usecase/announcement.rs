//! Announcement use cases — list and create team announcements.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::complex::announcement::{
    AnnouncementComplex, AnnouncementPermComplex,
};
use crate::data::announcement_data;
use crate::model::announcement_model;
use crate::model::user_model;
use crate::part::image::ImagePool;
use crate::part::repo::announcement::{
    AnnouncementRepo, AnnouncementRepoTransactional,
};
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::step::announcement::AnnouncementStep;
use crate::result::{RegularError, RegularResult};
use crate::util::DeriveTransactional;

#[cfg(test)]
mod tests;

/// Lists announcements under a team.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: user_model::Token,
    data: announcement_data::ListInfosData,
) -> RegularResult<Vec<announcement_data::InfoVal>>
where
    R: AnnouncementRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional:
        AnnouncementRepoTransactional<C> + MemberRepoTransactional<C>,
    I: ImagePool,
{
    let announcement_list_spec: announcement_model::ListSpec = data.into();

    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    AnnouncementPermComplex::can_user_list_infos(
        &mut repo.as_proxy(),
        &token.user_id,
        &announcement_list_spec.team_id,
    )
    .await?;

    let announcement_infos = repo
        .execute(&AnnouncementStep::list_infos(&announcement_list_spec))
        .await?;

    let mut announcement_info_vals =
        Vec::with_capacity(announcement_infos.len());

    for announcement_info in announcement_infos {
        announcement_info_vals.push(
            announcement_data::InfoVal::from_model(
                image_pool,
                announcement_info,
            )
            .await?,
        );
    }

    Ok(announcement_info_vals)
}

/// Creates an announcement under a team.
pub async fn create<D, C, R>(
    drive: &D,
    repo: &R,
    token: user_model::Token,
    data: announcement_data::CreateData,
) -> RegularResult<announcement_data::CreateVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: AnnouncementRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: AnnouncementRepoTransactional<C>
        + MemberRepoTransactional<C>
        + Send
        + Sync,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    AnnouncementPermComplex::can_user_create(
        &mut repo.as_proxy(),
        &token.user_id,
        &data.team_id,
    )
    .await?;

    let announcement_info = drive
        .with_context(
            async move |context| -> RegularResult<announcement_model::Info> {
                //
                let repo = repo.derive_transactional().await;

                let announcement_form = announcement_model::Form {
                    id: AnnouncementComplex::gen_id(),
                    team_id: data.team_id,
                    user_id: token.user_id,
                    title: data.title,
                    content: data.content,
                };

                let announcement_info = repo
                    .advance(
                        context,
                        &AnnouncementStep::create(&announcement_form),
                    )
                    .await?;

                Ok(announcement_info)
            },
        )
        .await?;

    Ok(announcement_data::CreateVal {
        id: announcement_info.id,
    })
}
