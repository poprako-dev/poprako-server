//! Complex-domain operations for comic entities: identity generation, cover-storage
//! key management, and recursive deletion with related resource cleanup.

use time::OffsetDateTime;
use uuid::Uuid;

use crate::complex::image::ImageComplex;
use crate::complex::util::{check_user_is_team_admin, check_user_is_team_member};
use crate::part::prom::intention::{IMAGE_TOPIC, ImageIntention};
use crate::part::prom::{Payload, PromStep, PromTransactional};
use crate::part::repo::comic::ComicRepoTransactional;
use crate::part::repo::proxy::ProxyExecute;
use crate::part::repo::step::comic::{ComicStep, GetInfoById as ComicGetInfoById};
use crate::part::repo::step::member::FindByUserTeamId;
use crate::part::repo::step::workset::{GetInfoById as WorksetGetInfoById, WorksetStep};
use crate::part::repo::workset::WorksetRepoTransactional;
use crate::result::{RootError, RootResult};

/// Domain operations for comic entities.
pub struct ComicComplex;

impl ComicComplex {
    pub fn gen_id() -> String {
        format!("comic-{}", Uuid::now_v7())
    }

    pub fn gen_cover_key(id: &str, cover_version: i64, file_ext: &str) -> String {
        format!("comic_cover/{}-{}.{}", id, cover_version, file_ext)
    }

    pub async fn delete_cascade<C, R, P>(
        repo: &R,
        prom: &P,
        context: &mut C,
        id: &str,
    ) -> RootResult<()>
    where
        C: Send,
        R: ComicRepoTransactional<C> + WorksetRepoTransactional<C> + Send + Sync,
        P: PromTransactional<C> + Send + Sync,
    {
        let comic_info = repo
            .advance(context, &ComicStep::get_info_excluded(id))
            .await?;

        if let Some(cover_key) = &comic_info.cover_key
            && comic_info.cover_uploaded
        {
            let delete_id = ImageComplex::gen_delete_id();
            let now = OffsetDateTime::now_utc();

            prom.advance(
                context,
                &PromStep::append(
                    &delete_id,
                    IMAGE_TOPIC,
                    Payload::Image(ImageIntention::Delete {
                        object_key: cover_key.clone(),
                    }),
                    &now,
                ),
            )
            .await?;
        }

        repo.advance(context, &ComicStep::delete(id)).await?;

        repo.advance(
            context,
            &WorksetStep::update_comic_count(&comic_info.workset_id, -1),
        )
        .await?;

        Ok(())
    }
}

/// Permission-gate operations for comic entities — comic-scoped.
pub struct ComicPermComplex;

impl ComicPermComplex {
    pub async fn can_user_create<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        let team_id = Self::resolve_team_id_from_workset(proxy, workset_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    pub async fn can_user_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        let team_id = Self::resolve_team_id_from_workset(proxy, workset_id).await?;

        check_user_is_team_member(proxy, user_id, &team_id).await
    }

    pub async fn can_user_get_info<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        let team_id = Self::resolve_team_id_from_comic(proxy, comic_id).await?;

        check_user_is_team_member(proxy, user_id, &team_id).await
    }

    pub async fn can_user_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        let team_id = Self::resolve_team_id_from_comic(proxy, comic_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    pub async fn can_user_reserve_cover<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        let team_id = Self::resolve_team_id_from_comic(proxy, comic_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    pub async fn can_user_mark_cover_uploaded<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        let team_id = Self::resolve_team_id_from_comic(proxy, comic_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    pub async fn can_user_delete<P>(proxy: &mut P, user_id: &str, comic_id: &str) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        let team_id = Self::resolve_team_id_from_comic(proxy, comic_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    pub async fn can_user_mark_completed<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        let team_id = Self::resolve_team_id_from_comic(proxy, comic_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    async fn resolve_team_id_from_workset<P>(proxy: &mut P, workset_id: &str) -> RootResult<String>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>,
    {
        let workset_info = proxy
            .execute(&WorksetStep::get_info_by_id(workset_id))
            .await?;

        Ok(workset_info.team_id)
    }

    async fn resolve_team_id_from_comic<P>(proxy: &mut P, comic_id: &str) -> RootResult<String>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>,
    {
        let comic_info = proxy.execute(&ComicStep::get_info_by_id(comic_id)).await?;

        let workset_info = proxy
            .execute(&WorksetStep::get_info_by_id(&comic_info.workset_id))
            .await?;

        Ok(workset_info.team_id)
    }
}
