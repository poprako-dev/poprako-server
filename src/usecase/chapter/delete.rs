//! Chapter deletion use case and cascade orchestration.

use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use crate::complex::chapter::ChapterPermComplex;
use crate::complex::image::ImageComplex;
use crate::model::shared::user::UserToken;
use crate::model::write::chapter::ChapterPatch;
use crate::part::nucl::Serializable;
use crate::part::prom::Prom;
use crate::part::prom::oper::DeferBatch;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::prom::task::Task;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::DeleteAssignments;
use crate::part::repo::oper::assignment_invitation::DeleteAssignmentInvitations;
use crate::part::repo::oper::chapter::{
    DeleteChapter, GetChapterInfoExcluded, ListChapterInfosExcluded,
    UnpinOtherChapters, UpdateChapter,
};
use crate::part::repo::oper::chapter_workflow_record::DeleteChapterWorkflowRecords;
use crate::part::repo::oper::comic::{
    TouchComicLastActive, UpdateComicChapterCount,
};
use crate::part::repo::oper::page::{DeletePages, ListPageInfos};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::unit::UnitRepo;
use crate::result::{BaseError, BaseRest, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;

/// Deletes one chapter and its descendant core records.
#[instrument(level = "info", skip(nucl, repo, prom))]
pub async fn delete<N, C, R, P>(
    (nucl, repo, prom): (&N, &R, &P),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context,
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    C::Level: AtLeast<Serializable>,
    R: ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + PageRepo<C>
        + AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    let member_info = MemberLoader::load_info_from_chapter(
        repo,
        LoadMode::<C>::Run,
        &token.user_id,
        &id,
    )
    .await?;

    ChapterPermComplex::ensure_user_can_delete(&member_info)?;

    nucl.coord(async move |context| {
        delete_cascade(repo, prom, context, &id).await
    })
    .await?;

    accept(())
}

/// Deletes a chapter subtree inside an existing transaction context.
pub async fn delete_cascade<C, R, P>(
    repo: &R,
    prom: &P,
    context: &mut C,
    id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: ChapterRepo<C>
        + PageRepo<C>
        + AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + Sync,
    P: Prom<C> + Sync,
{
    let chapter_info = GetChapterInfoExcluded { id, incls: &[] }
        .step_on(repo, context)
        .await?;

    defer_page_image_deletes(repo, prom, context, &chapter_info.id).await?;

    DeleteAssignmentInvitations::Chapter {
        chapter_id: &chapter_info.id,
    }
    .step_on(repo, context)
    .await?;

    DeleteAssignments::Chapter {
        chapter_id: &chapter_info.id,
    }
    .step_on(repo, context)
    .await?;

    DeleteChapterWorkflowRecords {
        chapter_id: &chapter_info.id,
    }
    .step_on(repo, context)
    .await?;

    DeletePages::Chapter {
        chapter_id: &chapter_info.id,
    }
    .step_on(repo, context)
    .await?;

    DeleteChapter {
        id: &chapter_info.id,
    }
    .step_on(repo, context)
    .await?;

    if chapter_info.is_pinned {
        repin_latest_chapter(repo, context, &chapter_info.comic_id).await?;
    }

    UpdateComicChapterCount {
        id: &chapter_info.comic_id,
        delta: -1,
    }
    .step_on(repo, context)
    .await?;

    TouchComicLastActive {
        id: &chapter_info.comic_id,
    }
    .step_on(repo, context)
    .await?;

    accept(())
}

// Load page image keys and enqueue their deletion inside the transaction.
async fn defer_page_image_deletes<C, R, P>(
    repo: &R,
    prom: &P,
    context: &mut C,
    chapter_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: PageRepo<C> + Sync,
    P: Prom<C> + Sync,
{
    let page_infos =
        ListPageInfos { chapter_id }.step_on(repo, context).await?;

    let object_keys = page_infos
        .into_iter()
        .filter_map(|page_info| page_info.image_key)
        .collect();

    defer_image_deletes(prom, context, object_keys).await
}

// Pin the newest remaining chapter after deleting the pinned chapter.
async fn repin_latest_chapter<C, R>(
    repo: &R,
    context: &mut C,
    comic_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: ChapterRepo<C> + Sync,
{
    let chapter_infos = ListChapterInfosExcluded { comic_id }
        .step_on(repo, context)
        .await?;

    let Some(chapter_info) = chapter_infos.first() else {
        return accept(());
    };

    let chapter_info_update = ChapterPatch {
        id: chapter_info.id.clone(),
        subtitle: None,
        pin: Some(true),
    };

    UpdateChapter {
        update: &chapter_info_update,
    }
    .step_on(repo, context)
    .await?;

    UnpinOtherChapters {
        comic_id: &chapter_info.comic_id,
        excluded_id: &chapter_info.id,
    }
    .step_on(repo, context)
    .await?;

    accept(())
}

// Enqueue object-storage image deletions inside the transaction.
async fn defer_image_deletes<C, P>(
    prom: &P,
    context: &mut C,
    object_keys: Vec<String>,
) -> BaseRest<()>
where
    C: Context,
    P: Prom<C> + Sync,
{
    let delete_ids = object_keys
        .iter()
        .map(|_| ImageComplex::gen_delete_id())
        .collect::<Vec<_>>();

    let payloads = object_keys
        .into_iter()
        .map(|object_key| TaskPayload::Image {
            payload: image::ImagePayload::Delete { object_key },
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

    DeferBatch::new(&tasks).step_on(prom, context).await?;

    accept(())
}
