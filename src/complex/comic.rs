//! Complex-domain opers for comic entities: identity generation,
//! cover-storage key management, and permission gates.

use time::OffsetDateTime;

use crate::complex::chapter::ChapterComplex;
use crate::complex::image::ImageComplex;
use crate::complex::util::{check_user_is_team_admin, check_user_is_team_member};
use crate::model::comic::ComicInfo;
use crate::part::prom::task::{IMAGE_TOPIC, ImageTask};
use crate::part::prom::{Payload, PromStep, PromTransactional};
use crate::part::repo::chapter::ChapterRepoTransactional;
use crate::part::repo::comic::ComicRepoTransactional;
use crate::part::repo::page::PageRepoTransactional;
use crate::part::repo::step::chapter::ChapterStep;
use crate::part::repo::step::comic::{ComicStep, GetInfoById as ComicGetInfoById};
use crate::part::repo::step::member::FindInfoByUserIdAndTeamId;
use crate::part::repo::step::workset::{GetInfoById as WorksetGetInfoById, WorksetStep};
use crate::part::repo::workset::WorksetRepoTransactional;
use crate::part::shared::proxy::ProxyExecute;
use crate::result::{RegularError, RegularResult, accept};
use crate::util::next_snowflake_id;

/// Domain opers for comic entities: identity generation and
/// cover-storage key management.
pub struct ComicComplex;

impl ComicComplex {
    /// Generate a unique, time-ordered comic identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Construct the object-storage key for a comic cover image.
    ///
    /// Format: `comic_cover/{id}-{version}.{ext}`.
    pub fn gen_cover_key(id: &str, cover_version: i64, file_ext: &str) -> String {
        format!("comic_cover/{}-{}.{}", id, cover_version, file_ext)
    }

    /// Compose a display title for fuzzy search, joining index, author, and title.
    ///
    /// Format: `"{index} {author} {title}"` — a single keyword can match any of the three fields.
    pub fn composed_title(info: &ComicInfo) -> String {
        format!("{} {} {}", info.index, info.author, info.title)
    }

    /// Deletes a comic subtree inside an existing transaction context.
    pub async fn delete_cascade<C, R, P>(
        repo: &R,
        prom: &P,
        context: &mut C,
        id: &str,
    ) -> RegularResult<()>
    where
        C: Send,
        R: ComicRepoTransactional<C>
            + WorksetRepoTransactional<C>
            + ChapterRepoTransactional<C>
            + PageRepoTransactional<C>
            + Send
            + Sync,
        P: PromTransactional<C> + Send + Sync,
    {
        let comic_info = repo
            .advance(context, &ComicStep::get_info_excluded(id))
            .await?;

        let chapter_infos = repo
            .advance(
                context,
                &ChapterStep::list_all_infos_by_comic_id_excluded(&comic_info.id),
            )
            .await?;

        for chapter_info in chapter_infos {
            ChapterComplex::delete_cascade(repo, prom, context, &chapter_info.id).await?;
        }

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
                    Payload::Image(ImageTask::Delete {
                        object_key: cover_key.as_str(),
                    }),
                    &now,
                ),
            )
            .await?;
        }

        repo.advance(context, &ComicStep::delete(&comic_info.id))
            .await?;

        repo.advance(
            context,
            &WorksetStep::update_comic_count(&comic_info.workset_id, -1),
        )
        .await?;

        accept(())
    }
}

/// Permission-gate opers for comic entities — comic-scoped.
pub struct ComicPermComplex;

impl ComicPermComplex {
    /// Verify the caller is a team admin of the owning workset's team.
    pub async fn can_user_create<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RegularError>,
    {
        let team_id = Self::resolve_team_id_from_workset(proxy, workset_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team member of the owning workset's team.
    pub async fn can_user_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RegularError>,
    {
        let team_id = Self::resolve_team_id_from_workset(proxy, workset_id).await?;

        check_user_is_team_member(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team member of the comic's team.
    pub async fn can_user_get_info<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RegularError>,
    {
        let team_id = Self::resolve_team_id_from_comic(proxy, comic_id).await?;

        check_user_is_team_member(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team admin of the comic's team.
    pub async fn can_user_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RegularError>,
    {
        let team_id = Self::resolve_team_id_from_comic(proxy, comic_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team admin of the comic's team.
    pub async fn can_user_reserve_cover<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RegularError>,
    {
        let team_id = Self::resolve_team_id_from_comic(proxy, comic_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team admin of the comic's team.
    pub async fn can_user_mark_cover_uploaded<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RegularError>,
    {
        let team_id = Self::resolve_team_id_from_comic(proxy, comic_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team admin of the comic's team.
    pub async fn can_user_delete<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RegularError>,
    {
        let team_id = Self::resolve_team_id_from_comic(proxy, comic_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team admin of the comic's team.
    pub async fn can_user_mark_completed<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RegularError>,
    {
        let team_id = Self::resolve_team_id_from_comic(proxy, comic_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    /// Resolve the owning team ID from a workset ID.
    async fn resolve_team_id_from_workset<P>(
        proxy: &mut P,
        workset_id: &str,
    ) -> RegularResult<String>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>,
    {
        let workset_info = proxy
            .execute(&WorksetStep::get_info_by_id(workset_id))
            .await?;

        Ok(workset_info.team_id)
    }

    /// Resolve the owning team ID from a comic ID (via its workset).
    async fn resolve_team_id_from_comic<P>(proxy: &mut P, comic_id: &str) -> RegularResult<String>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>,
    {
        let comic_info = proxy.execute(&ComicStep::get_info_by_id(comic_id)).await?;

        let workset_info = proxy
            .execute(&WorksetStep::get_info_by_id(&comic_info.workset_id))
            .await?;

        Ok(workset_info.team_id)
    }
}
