//! Announcement use cases — list and create team announcements.

use poprako_orchestra::{Nucl, run_proxy};
use tracing::instrument;

use crate::complex::announcement::{
    AnnouncementComplex, AnnouncementPermComplex,
};
use crate::data::announcement::{
    AnnouncementInfoVal, CreateAnnouncementParams, CreateAnnouncementPayload,
    ListAnnouncementInfosParams,
};
use crate::model::announcement::{AnnouncementEntry, AnnouncementListSpec};
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::repo::announcement::AnnouncementRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::announcement::{
    CreateAnnouncement, ListAnnouncementInfos,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::result::{BaseError, BaseResult, accept};

#[cfg(test)]
mod tests;

/// Lists announcements under a team.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    params: ListAnnouncementInfosParams,
) -> BaseResult<Vec<AnnouncementInfoVal>>
where
    R: AnnouncementRepo<C> + MemberRepo<C> + Sync,
    I: ImagePool,
{
    let announcement_list_spec: AnnouncementListSpec = params.into();

    AnnouncementPermComplex::ensure_user_can_list_infos(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &announcement_list_spec.team_id,
    )
    .await?;

    let announcement_infos = repo
        .run(&ListAnnouncementInfos {
            spec: &announcement_list_spec,
        })
        .await?;

    let mut announcement_info_vals =
        Vec::with_capacity(announcement_infos.len());

    for announcement_info in announcement_infos {
        announcement_info_vals.push(
            AnnouncementInfoVal::from_model(image_pool, announcement_info)
                .await?,
        );
    }

    accept(announcement_info_vals)
}

/// Creates an announcement under a team.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn create<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: CreateAnnouncementParams,
) -> BaseResult<CreateAnnouncementPayload>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: AnnouncementRepo<C> + MemberRepo<C> + Send + Sync,
{
    AnnouncementPermComplex::ensure_user_can_create(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &params.team_id,
    )
    .await?;

    let announcement_info = nucl
        .coord(async move |context| {
            //
            let announcement_entry = AnnouncementEntry {
                id: AnnouncementComplex::gen_id(),
                team_id: params.team_id,
                user_id: token.user_id,
                title: params.title,
                content: params.content,
            };

            repo.step(
                context,
                &CreateAnnouncement {
                    entry: &announcement_entry,
                },
            )
            .await
        })
        .await?;

    accept(CreateAnnouncementPayload {
        id: announcement_info.id,
    })
}
