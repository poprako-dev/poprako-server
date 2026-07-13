use poprako_orchestra_extra::prom::oper::DeferBatch;
use poprako_orchestra_extra::prom::task::Task;

use crate::complex::chapter::ChapterComplex;
use crate::complex::image::ImageComplex;
use crate::model::chapter::ChapterInfoUpdate;
use crate::part::prom::Prom;
use crate::part::prom::payload::{Payload, image};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
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
use crate::part::repo::page::PageRepo;
use crate::part::repo::unit::UnitRepo;
use crate::result::RegularResult;

impl ChapterComplex {
    /// Appends page image deletes inside an existing transaction context.
    pub async fn clean_uploaded_images<C, R, P>(
        repo: &R,
        prom: &P,
        context: &mut C,
        chapter_id: &str,
    ) -> RegularResult<()>
    where
        C: Send,
        R: PageRepo<C> + Send + Sync,
        P: Prom<C> + Send + Sync,
    {
        prom_image_deletes(repo, prom, context, chapter_id).await
    }

    /// Deletes a chapter subtree inside an existing transaction context.
    pub async fn delete_cascade<C, R, P>(
        repo: &R,
        prom: &P,
        context: &mut C,
        id: &str,
    ) -> RegularResult<()>
    where
        C: Send,
        R: ChapterRepo<C>
            + ComicRepo<C>
            + PageRepo<C>
            + AssignmentInvitationRepo<C>
            + AssignmentRepo<C>
            + UnitRepo<C>
            + Send
            + Sync,
        P: Prom<C> + Send + Sync,
    {
        // SAFETY: Lock the root chapter row (FOR UPDATE) to serialize with
        // concurrent page/unit insertions, preventing resource leaks from
        // pages (and their uploaded images) inserted between the listing
        // and the chapter delete.

        let chapter_info = repo
            .step(context, &GetChapterInfoExcluded { id, incls: &[] })
            .await?;

        prom_image_deletes(repo, prom, context, &chapter_info.id).await?;

        // Delete leaf FKs first to satisfy ON DELETE RESTRICT constraints.

        repo.step(
            context,
            &DeleteAssignmentInvitations::Chapter {
                chapter_id: &chapter_info.id,
            },
        )
        .await?;

        repo.step(
            context,
            &DeleteAssignments::Chapter {
                chapter_id: &chapter_info.id,
            },
        )
        .await?;

        // DeletePages::Chapter deletes units then pages internally.

        repo.step(
            context,
            &DeletePages::Chapter {
                chapter_id: &chapter_info.id,
            },
        )
        .await?;

        repo.step(
            context,
            &DeleteChapter {
                id: &chapter_info.id,
            },
        )
        .await?;

        if chapter_info.is_pinned {
            repin_latest_chapter(repo, context, &chapter_info.comic_id).await?;
        }

        repo.step(
            context,
            &UpdateComicChapterCount {
                id: &chapter_info.comic_id,
                delta: -1,
            },
        )
        .await?;

        repo.step(
            context,
            &TouchComicLastActive {
                id: &chapter_info.comic_id,
            },
        )
        .await?;

        Ok(())
    }
}

/// Schedule image deletion tasks for all uploaded page images belonging
/// to the given chapter.
async fn prom_image_deletes<C, R, P>(
    repo: &R,
    prom: &P,
    context: &mut C,
    chapter_id: &str,
) -> RegularResult<()>
where
    C: Send,
    R: PageRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
{
    let page_infos = repo
        .step(context, &ListPageInfos::AllChapter { chapter_id })
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

    let tasks: Vec<_> = delete_ids
        .iter()
        .zip(payloads.iter())
        .map(|(id, payload)| Task {
            id,
            payload,
            delay: None,
        })
        .collect();

    prom.step(context, &DeferBatch::new(&tasks)).await?;

    Ok(())
}

/// After deleting a pinned chapter, repin the most recent remaining chapter
/// (by list order) for the same comic.
async fn repin_latest_chapter<C, R>(
    repo: &R,
    context: &mut C,
    comic_id: &str,
) -> RegularResult<()>
where
    C: Send,
    R: ChapterRepo<C> + Send + Sync,
{
    let chapter_infos = repo
        .step(context, &ListChapterInfosExcluded { comic_id })
        .await?;

    let Some(chapter_info) = chapter_infos.first() else {
        return Ok(());
    };

    let chapter_info_update = ChapterInfoUpdate {
        id: chapter_info.id.clone(),
        subtitle: None,
        pin: Some(true),
    };

    repo.step(
        context,
        &UpdateChapter {
            update: &chapter_info_update,
        },
    )
    .await?;

    repo.step(
        context,
        &UnpinOtherChapters {
            comic_id: &chapter_info.comic_id,
            excluded_id: &chapter_info.id,
        },
    )
    .await?;

    Ok(())
}
