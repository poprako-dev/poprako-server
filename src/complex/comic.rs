//! Complex-domain opers for comic entities: identity generation,
//! cover-storage key management, and perm gates.

use std::collections::HashMap;

use poprako_orchestra::{OperProxy as _, Proxy};

use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::image::ImageComplex;
use crate::complex::termbase::TermbaseComplex;
use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_admin_with_roles,
    check_user_is_team_member,
};
use crate::model::read::proj::comic::ComicInfo;
use crate::part::prom::oper::{Defer, DeferBatch};
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::prom::task::Task;
use crate::part::repo::oper::assignment::DeleteAssignments;
use crate::part::repo::oper::assignment_invitation::DeleteAssignmentInvitations;
use crate::part::repo::oper::chapter::{
    DeleteChapter, GetChapterInfoExcluded, ListChapterInfosExcluded,
    ListPinnedChapterInfos, UnpinOtherChapters, UpdateChapter,
};
use crate::part::repo::oper::comic::{
    DeleteComic, GetComicInfoExcluded, TouchComicLastActive,
    UpdateComicChapterCount,
};
use crate::part::repo::oper::comic_archive::DeleteComicArchives;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::page::{
    DeletePages, ListFirstPageInfos, ListPageInfos,
};
use crate::part::repo::oper::team::ResolveTeamId;
use crate::part::repo::oper::term::DeleteTerms;
use crate::part::repo::oper::termbase::{
    DeleteTermbase, GetTermbaseInfoExcluded, ListTermbaseInfosExcluded,
};
use crate::part::repo::oper::workset::{
    GetWorksetInfo, UpdateWorksetComicCount,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::index::stored_index_to_user_index;
use crate::value::role::RoleMask;

/// Domain opers for comic entities: identity generation and
/// cover-storage key management.
pub struct ComicComplex;

impl ComicComplex {
    /// Rejects ordinary mutations after a comic has been archived.
    pub fn ensure_comic_writable(comic_info: &ComicInfo) -> BaseRest<()> {
        //
        if comic_info.archived_at.is_none() {
            return accept(());
        }

        let err_message = trl("error-comic-archived");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            comic_id = %comic_info.id,
            "expected error: archived comic is frozen",
        );

        Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        })
    }

    /// Generate a unique, time-ordered comic identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Construct the object-storage key for a comic cover image.
    ///
    /// Format: `comic_cover/{id}-{version}.{ext}`.
    pub fn gen_cover_key(id: &str, version: u32, file_ext: &str) -> String {
        format!("comic_cover/{}-{}.{}", id, version, file_ext)
    }

    /// Compose a display title from raw fields for search materialization.
    pub fn compose_title(index: i32, author: &str, title: &str) -> String {
        format!("{} {} {}", stored_index_to_user_index(index), author, title)
    }

    /// Resolve uploaded first-page image keys for the comics' pinned chapters.
    pub async fn resolve_fallback_cover_keys<P>(
        proxy: &mut P,
        comic_ids: &[String],
    ) -> BaseRest<HashMap<String, String>>
    where
        P: for<'a> Proxy<ListPinnedChapterInfos<'a>, Error = BaseError>
            + for<'a> Proxy<ListFirstPageInfos<'a>, Error = BaseError>,
    {
        if comic_ids.is_empty() {
            return accept(HashMap::new());
        }

        let pinned_chapter_infos =
            ListPinnedChapterInfos { comic_ids }.proxy_on(proxy).await?;

        let chapter_ids = pinned_chapter_infos
            .iter()
            .map(|chapter_info| chapter_info.id.clone())
            .collect::<Vec<_>>();

        let first_page_by_chapter = ListFirstPageInfos {
            chapter_ids: &chapter_ids,
        }
        .proxy_on(proxy)
        .await?
        .into_iter()
        .map(|page_info| (page_info.chapter_id.clone(), page_info))
        .collect::<HashMap<_, _>>();

        let mut fallback_cover_keys = HashMap::new();

        for chapter_info in pinned_chapter_infos {
            //
            let Some(page_info) = first_page_by_chapter.get(&chapter_info.id)
            else {
                continue;
            };

            let (Some(true), Some(image_key)) =
                (page_info.is_image_uploaded, &page_info.image_key)
            else {
                continue;
            };

            fallback_cover_keys
                .insert(chapter_info.comic_id, image_key.clone());
        }

        accept(fallback_cover_keys)
    }

    /// Deletes a comic subtree inside an existing transaction context.
    pub async fn delete_cascade<P>(proxy: &mut P, id: &str) -> BaseRest<()>
    where
        P: for<'a, 'b> Proxy<GetComicInfoExcluded<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<ListChapterInfosExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteComic<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteComicArchives<'a>, Error = BaseError>
            + for<'a> Proxy<UpdateWorksetComicCount<'a>, Error = BaseError>
            + for<'a, 'b> Proxy<GetChapterInfoExcluded<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<ListPageInfos<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteAssignmentInvitations<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteAssignments<'a>, Error = BaseError>
            + for<'a> Proxy<DeletePages<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteChapter<'a>, Error = BaseError>
            + for<'a> Proxy<UpdateChapter<'a>, Error = BaseError>
            + for<'a> Proxy<UnpinOtherChapters<'a>, Error = BaseError>
            + for<'a> Proxy<UpdateComicChapterCount<'a>, Error = BaseError>
            + for<'a> Proxy<TouchComicLastActive<'a>, Error = BaseError>
            + for<'a> Proxy<ListTermbaseInfosExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<GetTermbaseInfoExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTerms<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTermbase<'a>, Error = BaseError>
            + for<'a> Proxy<Defer<'a, String, TaskPayload, ()>, Error = BaseError>
            + for<'t, 'a> Proxy<
                DeferBatch<'t, 'a, String, TaskPayload, ()>,
                Error = BaseError,
            >,
    {
        // SAFETY: Lock the root comic row (FOR UPDATE) to serialize with
        // concurrent chapter creations and cover uploads, preventing resource
        // leaks from chapters (and their page images) inserted between the
        // listing and the comic delete.

        let comic_info = GetComicInfoExcluded { id, incls: &[] }
            .proxy_on(proxy)
            .await?;

        TermbaseComplex::delete_comic_cascade(proxy, &comic_info.id).await?;

        let chapter_infos = ListChapterInfosExcluded {
            comic_id: &comic_info.id,
        }
        .proxy_on(proxy)
        .await?;

        for chapter_info in chapter_infos {
            ChapterComplex::delete_cascade(proxy, &chapter_info.id).await?;
        }

        if let Some(cover_key) = &comic_info.cover_key
            && comic_info.is_cover_uploaded == Some(true)
        {
            let (delete_id, payload) = (
                ImageComplex::gen_delete_id(),
                TaskPayload::Image(image::ImagePayload::Delete {
                    object_key: cover_key.clone(),
                }),
            );

            let task = Task {
                id: &delete_id,
                payload: &payload,
                delay: None,
            };

            Defer::new(task).proxy_on(proxy).await?;
        }

        DeleteComicArchives {
            source_comic_id: &comic_info.id,
        }
        .proxy_on(proxy)
        .await?;

        DeleteComic { id: &comic_info.id }.proxy_on(proxy).await?;

        UpdateWorksetComicCount {
            id: &comic_info.workset_id,
            delta: -1,
        }
        .proxy_on(proxy)
        .await?;

        accept(())
    }
}

/// perm-gate opers for comic entities — comic-scoped.
pub struct ComicPermComplex;

impl ComicPermComplex {
    /// Verify the caller is a team admin of the owning workset's team.
    pub async fn ensure_user_can_create<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
        preset_assignment_roles: Option<RoleMask>,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let workset_info =
            GetWorksetInfo { id: workset_id }.proxy_on(proxy).await?;

        check_user_is_team_admin_with_roles(
            proxy,
            user_id,
            &workset_info.team_id,
            preset_assignment_roles,
        )
        .await
    }

    /// Verify the caller is a team member of the owning workset's team.
    pub async fn ensure_user_can_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let workset_info =
            GetWorksetInfo { id: workset_id }.proxy_on(proxy).await?;

        check_user_is_team_member(proxy, user_id, &workset_info.team_id).await
    }

    /// Verify the caller is a team member of the comic's team.
    pub async fn ensure_user_can_get_info<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let team_id = ResolveTeamId::Comic { id: comic_id }
            .proxy_on(proxy)
            .await?;

        check_user_is_team_member(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team admin of the comic's team.
    pub async fn ensure_user_can_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let team_id = ResolveTeamId::Comic { id: comic_id }
            .proxy_on(proxy)
            .await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team admin of the comic's team.
    pub async fn ensure_user_can_reserve_cover<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let team_id = ResolveTeamId::Comic { id: comic_id }
            .proxy_on(proxy)
            .await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team admin of the comic's team.
    pub async fn ensure_user_can_mark_cover_uploaded<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let team_id = ResolveTeamId::Comic { id: comic_id }
            .proxy_on(proxy)
            .await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team admin of the comic's team.
    pub async fn ensure_user_can_delete<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let team_id = ResolveTeamId::Comic { id: comic_id }
            .proxy_on(proxy)
            .await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }
}
