//! Complex-domain opers for comic entities: identity generation,
//! cover-storage key management, and permission gates.

use poprako_orchestra::Proxy;
use poprako_orchestra_extra::prom::oper::Defer;
use poprako_orchestra_extra::prom::task::Task;

use crate::complex::chapter::ChapterComplex;
use crate::complex::image::ImageComplex;
use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_member,
};
use crate::part::prom::Prom;
use crate::part::prom::payload::{Payload, image};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::chapter::ListChapterInfosExcluded;
use crate::part::repo::oper::comic::{
    DeleteComic, GetComicInfo, GetComicInfoExcluded,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::workset::{
    GetWorksetInfo, UpdateWorksetComicCount,
};
use crate::part::repo::page::PageRepo;
use crate::part::repo::unit::UnitRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{RegularError, RegularResult};
use crate::util::next_snowflake_id;
use crate::value::index::stored_index_to_user_index;

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
    pub fn gen_cover_key(id: &str, version: u32, file_ext: &str) -> String {
        // FIXME: change to cover/id/version/ext
        // All images needs fixes.
        format!("comic_cover/{}-{}.{}", id, version, file_ext)
    }

    /// Compose a display title from raw fields for search materialization.
    pub fn compose_title(index: i32, author: &str, title: &str) -> String {
        format!("{} {} {}", stored_index_to_user_index(index), author, title)
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
        R: ComicRepo<C>
            + WorksetRepo<C>
            + ChapterRepo<C>
            + PageRepo<C>
            + AssignmentInvitationRepo<C>
            + AssignmentRepo<C>
            + UnitRepo<C>
            + Send
            + Sync,
        P: Prom<C> + Send + Sync,
    {
        // SAFETY: Lock the root comic row (FOR UPDATE) to serialize with
        // concurrent chapter creations and cover uploads, preventing resource
        // leaks from chapters (and their page images) inserted between the
        // listing and the comic delete.

        let comic_info = repo
            .step(context, &GetComicInfoExcluded { id, incls: &[] })
            .await?;

        let chapter_infos = repo
            .step(
                context,
                &ListChapterInfosExcluded {
                    comic_id: &comic_info.id,
                },
            )
            .await?;

        for chapter_info in chapter_infos {
            ChapterComplex::delete_cascade(
                repo,
                prom,
                context,
                &chapter_info.id,
            )
            .await?;
        }

        if let Some(cover_key) = &comic_info.cover_key
            && comic_info.cover_uploaded
        {
            let delete_id = ImageComplex::gen_delete_id();

            let payload = Payload::Image(image::Payload::Delete {
                object_key: cover_key.clone(),
            });

            let task = Task {
                id: &delete_id,
                payload: &payload,
                delay: None,
            };

            prom.step(context, &Defer::new(task)).await?;
        }

        repo.step(context, &DeleteComic { id: &comic_info.id })
            .await?;

        repo.step(
            context,
            &UpdateWorksetComicCount {
                id: &comic_info.workset_id,
                delta: -1,
            },
        )
        .await?;

        Ok(())
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
        P: for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
    {
        let team_id =
            Self::resolve_team_id_from_workset(proxy, workset_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team member of the owning workset's team.
    pub async fn can_user_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
    {
        let team_id =
            Self::resolve_team_id_from_workset(proxy, workset_id).await?;

        check_user_is_team_member(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team member of the comic's team.
    pub async fn can_user_get_info<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = RegularError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
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
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = RegularError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
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
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = RegularError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
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
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = RegularError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
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
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = RegularError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
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
        P: for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>,
    {
        let workset_info =
            proxy.exec(&GetWorksetInfo { id: workset_id }).await?;

        Ok(workset_info.team_id)
    }

    /// Resolve the owning team ID from a comic ID (via its workset).
    async fn resolve_team_id_from_comic<P>(
        proxy: &mut P,
        comic_id: &str,
    ) -> RegularResult<String>
    where
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = RegularError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>,
    {
        let comic_info = proxy
            .exec(&GetComicInfo {
                id: comic_id,
                incls: &[],
            })
            .await?;

        let workset_info = proxy
            .exec(&GetWorksetInfo {
                id: &comic_info.workset_id,
            })
            .await?;

        Ok(workset_info.team_id)
    }
}
