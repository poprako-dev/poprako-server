//! Chapter deletion use case and cascade orchestration.

use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::{ObjDept, obj_inst};

use crate::complex::chapter::perm::ChapterPermComplex;
use crate::model::shared::user::UserToken;
use crate::model::write::chapter::ChapterPatch;
use crate::part::nucl::Serial;
use crate::part::obj_dept::PageImage;
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
#[instrument(level = "info", skip(nucl, repo, obj_dept))]
pub async fn delete<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<Serial>,
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
    O: ObjDept<PageImage, C> + Send + Sync,
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
        delete_cascade(repo, obj_dept, context, &id).await
    })
    .await?;

    accept(())
}

/// Deletes a chapter subtree inside an existing transaction context.
pub async fn delete_cascade<C, R, O>(
    repo: &R,
    obj_dept: &O,
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
    O: ObjDept<PageImage, C> + Sync,
{
    let chapter_info = GetChapterInfoExcluded { id, incls: &[] }
        .step_on(repo, context)
        .await?;

    remove_page_objs(repo, obj_dept, context, &chapter_info.id).await?;

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
async fn remove_page_objs<C, R, O>(
    repo: &R,
    obj_dept: &O,
    context: &mut C,
    chapter_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: PageRepo<C> + Sync,
    O: ObjDept<PageImage, C> + Sync,
{
    let page_infos =
        ListPageInfos { chapter_id }.step_on(repo, context).await?;

    let page_ids = page_infos
        .into_iter()
        .map(|page_info| page_info.id)
        .collect::<Vec<_>>();

    obj_inst! { DelObjs<PageImage>::Remove { ids: &page_ids } }
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)
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
