//! Comic use cases — create, read, update, cover management, and deletion.

/// Comic listing use cases.
pub mod list;
/// Cover reservation use case.
pub mod reserve;

/// Comic use-case test helpers.
#[cfg(test)]
pub mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::assignment::AssignmentComplex;
use crate::complex::chapter::ChapterComplex;
use crate::complex::comic::{ComicComplex, ComicPermComplex};
use crate::complex::image::ImageComplex;
use crate::data::instr::comic::{
    CreateComicInstr, MarkComicCoverUploadedInstr, UpdateComicInfoInstr,
};
use crate::data::val::comic::CreateComicVal;
use crate::data::view::comic::ComicInfoView;
use crate::model::shared::user::UserToken;
use crate::model::write::assignment::AssignmentEntry;
use crate::model::write::chapter::ChapterEntry;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::model::write::comic::{ComicEntry, ComicRepl};
use crate::part::image::{ImageManager, ImagePool};
use crate::part::nucl::{RepeatableRead, Serializable};
use crate::part::prom::Prom;
use crate::part::prom::oper::Defer;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::prom::task::Task;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::comic_archive::ComicArchiveRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::CreateAssignment;
use crate::part::repo::oper::chapter::{
    CreateChapter, ListChapterInfosExcluded, UnpinOtherChapters,
};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic::{
    AllocComicChapterIndex, CreateComic, DeleteComic, GetComicInfo,
    GetComicInfoExcluded, MarkComicCoverUploaded, TouchComicLastActive,
    UpdateComic, UpdateComicChapterCount,
};
use crate::part::repo::oper::comic_archive::DeleteComicArchives;
use crate::part::repo::oper::workset::{
    AllocWorksetComicIndex, UpdateWorksetComicCount,
};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::term::TermRepo;
use crate::part::repo::termbase::TermbaseRepo;
use crate::part::repo::unit::UnitRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::chapter::delete::delete_cascade as delete_chapter_cascade;
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::page::PageLoader;
use crate::usecase::internal::util::LoadMode;
use crate::usecase::termbase::delete_comic_cascade;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;

/// Creates a new comic inside a workset together with its first
/// chapter and a creator admin assignment.
///
/// Inside a single transaction this:
/// 1. Allocates a workset-scoped comic index.
/// 2. Inserts the comic row.
/// 3. Bumps the workset comic count.
/// 4. Allocates a chapter index and inserts the first (pinned) chapter.
/// 5. Updates the comic's denormalised chapter counter and last-activity
///    timestamp.
/// 6. Creates an ADMIN assignment on the new chapter for the caller.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: CreateComicInstr,
) -> BaseRest<CreateComicVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<RepeatableRead>,
    R: ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + AssignmentRepo<C>
        + Send
        + Sync,
{
    let member_info = MemberLoader::load_info_from_workset(
        repo,
        LoadMode::Run,
        &token.user_id,
        &instr.workset_id,
    )
    .await?;

    ComicPermComplex::ensure_user_can_create(
        &member_info,
        instr.preset_assignment_roles,
    )?;

    let (comic_id, chapter_id) = nucl
        .coord(async move |context| {
            //
            let index = AllocWorksetComicIndex {
                id: &instr.workset_id,
            }
            .step_on(repo, context)
            .await?;

            let comic_entry = ComicEntry {
                id: ComicComplex::gen_id(),
                workset_id: instr.workset_id,
                index,
                title: instr.title,
                author: instr.author,
                description: instr.description,
                creator_id: token.user_id.clone(),
            };

            let comic_info = CreateComic {
                entry: &comic_entry,
            }
            .step_on(repo, context)
            .await?;

            UpdateWorksetComicCount {
                id: &comic_entry.workset_id,
                delta: 1,
            }
            .step_on(repo, context)
            .await?;

            let chapter_index = AllocComicChapterIndex { id: &comic_info.id }
                .step_on(repo, context)
                .await?;

            let subtitle = ChapterComplex::subtitle_or_default(
                instr.first_chapter_subtitle,
                chapter_index,
            );

            let chapter_entry = ChapterEntry {
                id: ChapterComplex::gen_id(),
                comic_id: comic_info.id.clone(),
                is_pinned: true,
                index: chapter_index,
                subtitle,
                creator_id: token.user_id.clone(),
            };

            let chapter_info = CreateChapter {
                entry: &chapter_entry,
            }
            .step_on(repo, context)
            .await?;

            UnpinOtherChapters {
                comic_id: &chapter_info.comic_id,
                excluded_id: &chapter_info.id,
            }
            .step_on(repo, context)
            .await?;

            UpdateComicChapterCount {
                id: &chapter_info.comic_id,
                delta: 1,
            }
            .step_on(repo, context)
            .await?;

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            let assignment_entry = AssignmentEntry {
                id: AssignmentComplex::gen_id(),
                chapter_id: chapter_info.id.clone(),
                user_id: token.user_id.clone(),
                roles: AssignmentComplex::creator_roles(
                    instr.preset_assignment_roles,
                ),
            };

            CreateAssignment {
                entry: &assignment_entry,
            }
            .step_on(repo, context)
            .await?;

            let workflow_record_entry = ChapterWorkflowRecordEntry::new(
                chapter_info.id.clone(),
                Some(token.user_id),
                ChapterWorkflowRecordPayload::ChapterCreated,
            );

            CreateChapterWorkflowRecords {
                entries: std::slice::from_ref(&workflow_record_entry),
            }
            .step_on(repo, context)
            .await?;

            accept((comic_info.id, chapter_info.id))
        })
        .await?;

    accept(CreateComicVal {
        id: comic_id,
        chapter_id,
    })
}

/// Fetches a comic by ID with cover URL resolution.
#[instrument(level = "info", skip(repo, image_pool))]
pub async fn get_info<C, R, I>(
    (repo, image_pool): (&R, &I),
    token: UserToken,
    id: String,
) -> BaseRest<ComicInfoView>
where
    C: Context,
    R: ComicRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + ChapterRepo<C>
        + PageRepo<C>
        + Sync,
    I: ImagePool,
{
    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::Run,
        &token.user_id,
        &id,
    )
    .await?;

    ComicPermComplex::ensure_user_can_get_info(&member_info)?;

    let comic_info = GetComicInfo {
        id: &id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    let first_page_infos = PageLoader::load_infos_from_comics(
        repo,
        std::slice::from_ref(&comic_info.id),
    )
    .await?;

    let fallback_cover_keys =
        ComicComplex::resolve_fallback_cover_keys(first_page_infos);

    ComicInfoView::from_model(
        image_pool,
        comic_info,
        fallback_cover_keys.get(&id).map(String::as_str),
    )
    .await
}

/// Updates a comic's title, author, and description.
#[instrument(level = "info", skip(repo))]
pub async fn update_info<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: UpdateComicInfoInstr,
) -> BaseRest<()>
where
    C: Context,
    R: ComicRepo<C> + TeamRepo<C> + MemberRepo<C> + Sync,
{
    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::Run,
        &token.user_id,
        &instr.id,
    )
    .await?;

    ComicPermComplex::ensure_user_can_update_info(&member_info)?;

    let comic_info = GetComicInfo {
        id: &instr.id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    ComicComplex::ensure_comic_writable(&comic_info)?;

    let comic_info_update = ComicRepl {
        id: instr.id,
        title: instr.title,
        author: instr.author,
        description: instr.description,
    };

    UpdateComic {
        update: &comic_info_update,
    }
    .run_on(repo)
    .await?;

    accept(())
}

/// Marks a reserved comic cover as successfully uploaded.
#[instrument(level = "info", skip(nucl, repo, image_manager))]
pub async fn mark_cover_uploaded<N, C, R, I>(
    (nucl, repo, image_manager): (&N, &R, &I),
    token: UserToken,
    id: String,
    instr: MarkComicCoverUploadedInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<RepeatableRead>,
    R: ComicRepo<C> + TeamRepo<C> + MemberRepo<C> + Send + Sync,
    I: ImageManager,
{
    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::Run,
        &token.user_id,
        &id,
    )
    .await?;

    ComicPermComplex::ensure_user_can_mark_cover_uploaded(&member_info)?;

    let comic_info = GetComicInfo {
        id: &id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    ComicComplex::ensure_comic_writable(&comic_info)?;

    if comic_info.cover_version != Some(instr.image_version) {
        //
        let err_message = trl("error-stale-cover-upload");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            comic_id = %id,
            user_id = %token.user_id,
            image_version = instr.image_version,
            stored_image_version = comic_info.cover_version,
            "expected error: stale comic cover upload",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    if comic_info.is_cover_uploaded == Some(true) {
        return accept(());
    }

    let cover_key = comic_info.cover_key.clone().ok_or_else(|| {
        //
        let err_message = trl("error-stale-cover-upload");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            comic_id = %id,
            user_id = %token.user_id,
            image_version = instr.image_version,
            stored_image_version = comic_info.cover_version,
            "expected error: stale comic cover upload",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        }
    })?;

    if !image_manager.object_exists(&cover_key).await? {
        //
        let err_message = trl("error-stale-cover-upload");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            comic_id = %id,
            user_id = %token.user_id,
            image_version = instr.image_version,
            cover_key = %cover_key,
            "expected error: stale comic cover upload",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    nucl.coord(async move |context| {
        //
        let locked_comic_info = GetComicInfoExcluded {
            id: &id,
            incls: &[],
        }
        .step_on(repo, context)
        .await?;

        ComicComplex::ensure_comic_writable(&locked_comic_info)?;

        if locked_comic_info.cover_version != Some(instr.image_version)
            || locked_comic_info.cover_key.as_deref() != Some(&cover_key)
        {
            let err_message = trl("error-stale-cover-upload");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                comic_id = %id,
                user_id = %token.user_id,
                image_version = instr.image_version,
                locked_image_version = locked_comic_info.cover_version,
                cover_key = %cover_key,
                "expected error: stale comic cover upload",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        MarkComicCoverUploaded {
            id: &id,
            cover_version: instr.image_version,
            cover_key: Some(&cover_key),
            cover_uploaded: true,
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Deletes a comic and updates the parent workset counter.
#[instrument(level = "info", skip(nucl, repo, prom))]
pub async fn delete<N, C, R, P>(
    (nucl, repo, prom): (&N, &R, &P),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<Serializable>,
    R: ComicRepo<C>
        + ComicArchiveRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + PageRepo<C>
        + AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + UnitRepo<C>
        + TermbaseRepo<C>
        + TermRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::Run,
        &token.user_id,
        &id,
    )
    .await?;

    ComicPermComplex::ensure_user_can_delete(&member_info)?;

    nucl.coord(async move |context| {
        delete_cascade(repo, prom, context, &id).await
    })
    .await?;

    accept(())
}

/// Deletes a comic subtree inside an existing transaction context.
pub async fn delete_cascade<C, R, P>(
    repo: &R,
    prom: &P,
    context: &mut C,
    id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: ComicRepo<C>
        + ComicArchiveRepo<C>
        + WorksetRepo<C>
        + ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + PageRepo<C>
        + AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + TermbaseRepo<C>
        + TermRepo<C>
        + Sync,
    P: Prom<C> + Sync,
{
    let comic_info = GetComicInfoExcluded { id, incls: &[] }
        .step_on(repo, context)
        .await?;

    delete_comic_cascade(repo, context, &comic_info.id).await?;

    let chapter_infos = ListChapterInfosExcluded {
        comic_id: &comic_info.id,
    }
    .step_on(repo, context)
    .await?;

    for chapter_info in chapter_infos {
        delete_chapter_cascade(repo, prom, context, &chapter_info.id).await?;
    }

    if let Some(cover_key) = &comic_info.cover_key
        && comic_info.is_cover_uploaded == Some(true)
    {
        let delete_id = ImageComplex::gen_delete_id();

        let payload = TaskPayload::Image {
            payload: image::ImagePayload::Delete {
                object_key: cover_key.clone(),
            },
        };

        let task = Task {
            id: &delete_id,
            payload: &payload,
            delay: None,
        };

        Defer::new(task).step_on(prom, context).await?;
    }

    DeleteComicArchives {
        source_comic_id: &comic_info.id,
    }
    .step_on(repo, context)
    .await?;

    DeleteComic { id: &comic_info.id }
        .step_on(repo, context)
        .await?;

    UpdateWorksetComicCount {
        id: &comic_info.workset_id,
        delta: -1,
    }
    .step_on(repo, context)
    .await?;

    accept(())
}
