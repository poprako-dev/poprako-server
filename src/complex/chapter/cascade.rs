use poprako_orchestra::Proxy;
use poprako_orchestra_extra::prom::oper::DeferBatch;
use poprako_orchestra_extra::prom::task::Task;

use crate::complex::chapter::ChapterComplex;
use crate::complex::image::ImageComplex;
use crate::model::chapter::ChapterInfoUpdate;
use crate::part::prom::payload::{Payload, image};
use crate::part::repo::oper::assignment::DeleteAssignments;
use crate::part::repo::oper::assignment_invitation::DeleteAssignmentInvitations;
use crate::part::repo::oper::chapter::{
    DeleteChapter, GetChapterInfoExcluded, ListChapterInfosExcluded,
    UnpinOtherChapters, UpdateChapter,
};
use crate::part::repo::oper::comic::{
    TouchComicLastActive, UpdateComicChapterCount,
};
use crate::part::repo::oper::page::{DeletePages, ListPageInfos};
use crate::result::{RegularError, RegularResult};

impl ChapterComplex {
    /// Appends page image deletes inside an existing transaction context.
    pub async fn clean_uploaded_images<P>(
        proxy: &mut P,
        chapter_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> Proxy<ListPageInfos<'a>, Error = RegularError>
            + for<'t, 'a> Proxy<
                DeferBatch<'t, 'a, String, Payload, ()>,
                Error = RegularError,
            >,
    {
        prom_image_deletes(proxy, chapter_id).await
    }

    /// Deletes a chapter subtree inside an existing transaction context.
    pub async fn delete_cascade<P>(proxy: &mut P, id: &str) -> RegularResult<()>
    where
        P: for<'a, 'b> Proxy<
                GetChapterInfoExcluded<'a, 'b>,
                Error = RegularError,
            > + for<'a> Proxy<ListPageInfos<'a>, Error = RegularError>
            + for<'a> Proxy<DeleteAssignmentInvitations<'a>, Error = RegularError>
            + for<'a> Proxy<DeleteAssignments<'a>, Error = RegularError>
            + for<'a> Proxy<DeletePages<'a>, Error = RegularError>
            + for<'a> Proxy<DeleteChapter<'a>, Error = RegularError>
            + for<'a> Proxy<ListChapterInfosExcluded<'a>, Error = RegularError>
            + for<'a> Proxy<UpdateChapter<'a>, Error = RegularError>
            + for<'a> Proxy<UnpinOtherChapters<'a>, Error = RegularError>
            + for<'a> Proxy<UpdateComicChapterCount<'a>, Error = RegularError>
            + for<'a> Proxy<TouchComicLastActive<'a>, Error = RegularError>
            + for<'t, 'a> Proxy<
                DeferBatch<'t, 'a, String, Payload, ()>,
                Error = RegularError,
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

        Ok(())
    }
}

/// Schedule image deletion tasks for all uploaded page images belonging
/// to the given chapter.
async fn prom_image_deletes<P>(
    proxy: &mut P,
    chapter_id: &str,
) -> RegularResult<()>
where
    P: for<'a> Proxy<ListPageInfos<'a>, Error = RegularError>
        + for<'t, 'a> Proxy<
            DeferBatch<'t, 'a, String, Payload, ()>,
            Error = RegularError,
        >,
{
    let page_infos = proxy
        .exec(&ListPageInfos::AllChapter { chapter_id })
        .await?;

    let mut delete_ids = Vec::new();

    let mut payloads = Vec::new();

    for page_info in page_infos {
        if let Some(image_key) = page_info.image_key
            && page_info.image_uploaded
        {
            delete_ids.push(ImageComplex::gen_delete_id());

            payloads.push(Payload::Image(image::Payload::Delete {
                object_key: image_key,
            }));
        }
    }

    let tasks: Vec<Task<'_, String, Payload>> = delete_ids
        .iter()
        .zip(payloads.iter())
        .map(|(id, payload)| Task {
            id,
            payload,
            delay: None,
        })
        .collect();

    proxy.exec(&DeferBatch::new(&tasks)).await?;

    Ok(())
}

/// After deleting a pinned chapter, repin the most recent remaining chapter
/// (by list order) for the same comic.
async fn repin_latest_chapter<P>(
    proxy: &mut P,
    comic_id: &str,
) -> RegularResult<()>
where
    P: for<'a> Proxy<ListChapterInfosExcluded<'a>, Error = RegularError>
        + for<'a> Proxy<UpdateChapter<'a>, Error = RegularError>
        + for<'a> Proxy<UnpinOtherChapters<'a>, Error = RegularError>,
{
    let chapter_infos =
        proxy.exec(&ListChapterInfosExcluded { comic_id }).await?;

    let Some(chapter_info) = chapter_infos.first() else {
        return Ok(());
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

    Ok(())
}
