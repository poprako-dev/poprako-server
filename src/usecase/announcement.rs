//! Announcement use cases — list and create team announcements.

use poprako_orchestra::{Nucl, run_proxy};

use crate::complex::announcement::{
    AnnouncementComplex, AnnouncementPermComplex,
};
use crate::data::announcement::AnnouncementInfoVal;
use crate::data::announcement::CreateAnnouncementParams;
use crate::data::announcement::CreateAnnouncementPayload;
use crate::data::announcement::ListAnnouncementInfosParams;
use crate::model::announcement::AnnouncementEntry;
use crate::model::announcement::AnnouncementListSpec;
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::repo::announcement::AnnouncementRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::announcement::{
    CreateAnnouncement, ListAnnouncementInfos,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::result::{RegularError, RegularResult};

#[cfg(test)]
mod tests;

/// Lists announcements under a team.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    params: ListAnnouncementInfosParams,
) -> RegularResult<Vec<AnnouncementInfoVal>>
where
    R: AnnouncementRepo<C> + MemberRepo<C> + Sync,
    I: ImagePool,
{
    let announcement_list_spec: AnnouncementListSpec = params.into();

    AnnouncementPermComplex::can_user_list_infos(
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

    Ok(announcement_info_vals)
}

/// Creates an announcement under a team.
pub async fn create<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: CreateAnnouncementParams,
) -> RegularResult<CreateAnnouncementPayload>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: AnnouncementRepo<C> + MemberRepo<C> + Send + Sync,
{
    AnnouncementPermComplex::can_user_create(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &params.team_id,
    )
    .await?;

    let announcement_info = nucl
        .coord(async move |context| {
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

    Ok(CreateAnnouncementPayload {
        id: announcement_info.id,
    })
}
