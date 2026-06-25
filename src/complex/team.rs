use time::OffsetDateTime;
use uuid::Uuid;

use crate::complex::image::ImageComplex;
use crate::complex::workset::WorksetComplex;
use crate::part::prom::intention::{IMAGE_TOPIC, ImageIntention};
use crate::part::prom::{Payload, PromStep, PromTransactional};
use crate::part::repo::comic::ComicRepoTransactional;
use crate::part::repo::step::team::TeamStep;
use crate::part::repo::step::workset::WorksetStep;
use crate::part::repo::team::TeamRepoTransactional;
use crate::part::repo::workset::WorksetRepoTransactional;
use crate::result::RootResult;

pub struct TeamComplex;

impl TeamComplex {
    pub fn gen_id() -> String {
        format!("team-{}", Uuid::now_v7())
    }

    pub fn gen_avatar_key(id: &str, avatar_version: i64, file_ext: &str) -> String {
        format!("team_avatar/{}-{}.{}", id, avatar_version, file_ext)
    }

    pub async fn delete_cascade<C, R, P>(
        repo: &R,
        prom: &P,
        context: &mut C,
        id: &str,
    ) -> RootResult<()>
    where
        C: Send,
        R: TeamRepoTransactional<C> + WorksetRepoTransactional<C> + ComicRepoTransactional<C> + Send + Sync,
        P: PromTransactional<C> + Send + Sync,
    {
        let team_info = repo
            .advance(context, &TeamStep::get_info_excluded(id))
            .await?;

        let workset_infos = repo
            .advance(context, &WorksetStep::list_by_team_id_excluded(id))
            .await?;

        for workset_info in workset_infos {
            WorksetComplex::delete_cascade(repo, prom, context, &workset_info.id).await?;
        }

        if let Some(avatar_key) = &team_info.avatar_key
            && team_info.avatar_uploaded
        {
            let delete_id = ImageComplex::gen_delete_id();
            let now = OffsetDateTime::now_utc();

            prom.advance(
                context,
                &PromStep::append(
                    &delete_id,
                    IMAGE_TOPIC,
                    Payload::Image(ImageIntention::Delete {
                        object_key: avatar_key.clone(),
                    }),
                    &now,
                ),
            )
            .await?;
        }

        repo.advance(context, &TeamStep::delete(id)).await?;

        Ok(())
    }
}
