use poprako_orchestra::Proxy;
use poprako_orchestra_extra::prom::oper::DeferBatch;
use poprako_orchestra_extra::prom::task::Task;

use crate::complex::chapter::ChapterComplex;
use crate::complex::image::ImageComplex;
use crate::model::chapter::ChapterInfoUpdate;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::repo::oper::assignment::DeleteAssignments;
use crate::part::repo::oper::assignment_invitation::DeleteAssignmentInvitations;
use crate::part::repo::oper::chapter::{
    DeleteChapter, GetChapterInfoExcluded, ListChapterInfosExcluded,
    UnpinOtherChapters, UpdateChapter,
};
use crate::part::repo::oper::comic::{
    TouchComicLastActive, UpdateComicChapterCount,
};
use crate::part::repo::oper::page::{
    ClearPageImagesForPublish, DeletePages, ListPageInfos,
};
use crate::result::{BaseError, BaseResult, accept};

impl ChapterComplex {
    /// Appends page image deletes inside an existing transaction context.
    pub async fn clean_uploaded_images<P>(
        proxy: &mut P,
        chapter_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a> Proxy<ClearPageImagesForPublish<'a>, Error = BaseError>
            + for<'t, 'a> Proxy<
                DeferBatch<'t, 'a, String, TaskPayload, ()>,
                Error = BaseError,
            >,
    {
        let object_keys = proxy
            .exec(&ClearPageImagesForPublish { chapter_id })
            .await?;

        defer_image_deletes(proxy, object_keys).await
    }

    /// Deletes a chapter subtree inside an existing transaction context.
    pub async fn delete_cascade<P>(proxy: &mut P, id: &str) -> BaseResult<()>
    where
        P: for<'a, 'b> Proxy<GetChapterInfoExcluded<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<ListPageInfos<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteAssignmentInvitations<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteAssignments<'a>, Error = BaseError>
            + for<'a> Proxy<DeletePages<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteChapter<'a>, Error = BaseError>
            + for<'a> Proxy<ListChapterInfosExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<UpdateChapter<'a>, Error = BaseError>
            + for<'a> Proxy<UnpinOtherChapters<'a>, Error = BaseError>
            + for<'a> Proxy<UpdateComicChapterCount<'a>, Error = BaseError>
            + for<'a> Proxy<TouchComicLastActive<'a>, Error = BaseError>
            + for<'t, 'a> Proxy<
                DeferBatch<'t, 'a, String, TaskPayload, ()>,
                Error = BaseError,
            >,
    {
        // SAFETY: Lock the root chapter row (FOR UPDATE) to serialize with
        // concurrent page/unit insertions, preventing resource leaks from
        // pages (and their uploaded images) inserted between the listing
        // and the chapter delete.

        let chapter_info = proxy
            .exec(&GetChapterInfoExcluded { id, incls: &[] })
            .await?;

        prom_image_deletes(proxy, &chapter_info.id).await?;

        // Delete leaf FKs first to satisfy ON DELETE RESTRICT constraints.

        proxy
            .exec(&DeleteAssignmentInvitations::Chapter {
                chapter_id: &chapter_info.id,
            })
            .await?;

        proxy
            .exec(&DeleteAssignments::Chapter {
                chapter_id: &chapter_info.id,
            })
            .await?;

        // DeletePages::Chapter deletes units then pages internally.

        proxy
            .exec(&DeletePages::Chapter {
                chapter_id: &chapter_info.id,
            })
            .await?;

        proxy
            .exec(&DeleteChapter {
                id: &chapter_info.id,
            })
            .await?;

        if chapter_info.is_pinned {
            repin_latest_chapter(proxy, &chapter_info.comic_id).await?;
        }

        proxy
            .exec(&UpdateComicChapterCount {
                id: &chapter_info.comic_id,
                delta: -1,
            })
            .await?;

        proxy
            .exec(&TouchComicLastActive {
                id: &chapter_info.comic_id,
            })
            .await?;

        accept(())
    }
}

// Build and schedule image delete tasks for a collected object-key list.
// Schedule concrete image delete payloads for deletion workers.
async fn defer_image_deletes<P>(
    proxy: &mut P,
    object_keys: Vec<String>,
) -> BaseResult<()>
where
    P: for<'t, 'a> Proxy<
            DeferBatch<'t, 'a, String, TaskPayload, ()>,
            Error = BaseError,
        >,
{
    let delete_ids = object_keys
        .iter()
        .map(|_| ImageComplex::gen_delete_id())
        .collect::<Vec<_>>();

    let payloads = object_keys
        .into_iter()
        .map(|object_key| {
            TaskPayload::Image(image::ImagePayload::Delete { object_key })
        })
        .collect::<Vec<_>>();

    let tasks = delete_ids
        .iter()
        .zip(payloads.iter())
        .map(|(id, payload)| Task {
            id,
            payload,
            delay: None,
        })
        .collect::<Vec<Task<'_, String, TaskPayload>>>();

    proxy.exec(&DeferBatch::new(&tasks)).await?;

    accept(())
}

// Schedule image deletion tasks for all uploaded page images belonging
// to the given chapter.
async fn prom_image_deletes<P>(
    proxy: &mut P,
    chapter_id: &str,
) -> BaseResult<()>
where
    P: for<'a> Proxy<ListPageInfos<'a>, Error = BaseError>
        + for<'t, 'a> Proxy<
            DeferBatch<'t, 'a, String, TaskPayload, ()>,
            Error = BaseError,
        >,
{
    let page_infos = proxy.exec(&ListPageInfos { chapter_id }).await?;

    let object_keys = page_infos
        .into_iter()
        .filter_map(|page_info| page_info.image_key)
        .collect();

    defer_image_deletes(proxy, object_keys).await
}

// After deleting a pinned chapter, repin the most recent remaining chapter
// (by list order) for the same comic.
async fn repin_latest_chapter<P>(
    proxy: &mut P,
    comic_id: &str,
) -> BaseResult<()>
where
    P: for<'a> Proxy<ListChapterInfosExcluded<'a>, Error = BaseError>
        + for<'a> Proxy<UpdateChapter<'a>, Error = BaseError>
        + for<'a> Proxy<UnpinOtherChapters<'a>, Error = BaseError>,
{
    let chapter_infos =
        proxy.exec(&ListChapterInfosExcluded { comic_id }).await?;

    let Some(chapter_info) = chapter_infos.first() else {
        return accept(());
    };

    let chapter_info_update = ChapterInfoUpdate {
        id: chapter_info.id.clone(),
        subtitle: None,
        pin: Some(true),
    };

    proxy
        .exec(&UpdateChapter {
            update: &chapter_info_update,
        })
        .await?;

    proxy
        .exec(&UnpinOtherChapters {
            comic_id: &chapter_info.comic_id,
            excluded_id: &chapter_info.id,
        })
        .await?;

    accept(())
}
