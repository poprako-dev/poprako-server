//! Chapter use cases — list, read, create, update, and deletion.

/// Chapter deletion use cases.
pub mod delete;
/// Chapter workflow stage mutation use case.
pub mod stage;
/// Chapter presentation assembly.
pub mod view;
/// Immutable workflow record listing use case.
pub mod workflow_record;

#[cfg(test)]
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;

use crate::complex::chapter::perm as chapter_perm_complex;
use crate::complex::{chapter as chapter_complex, comic as comic_complex};
use crate::data::instr::chapter::{
    CreateChapterInstr, ListChapterInfosInstr, UpdateChapterInfoInstr,
};
use crate::data::val::chapter::CreateChapterVal;
use crate::data::view::chapter::ChapterInfoView;
use crate::model::read::spec::chapter::ChapterListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::chapter::ChapterPatch;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar, UserAvatar};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::chapter::{
    FindPinnedChapterInfo, GetChapterInfo, GetChapterInfoExcluded,
    ListChapterInfos, LockChapters, UnpinOtherChapters, UpdateChapter,
};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic::{
    GetComicInfoExcluded, TouchComicLastActive,
};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, accept};
use crate::usecase::chapter::view::chapter_info_views;
use crate::usecase::internal::chapter_creation as chapter_creation_usecase;
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;

/// Lists chapters under one comic.
#[instrument(level = "info", skip(repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn list_infos<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    instr: ListChapterInfosInstr,
) -> BaseRest<Vec<ChapterInfoView>>
where
    C: Context,
    R: ChapterRepo<C> + MemberRepo<C> + TeamRepo<C> + PageRepo<C> + Sync,
    O: ObjDeptView<ComicCover, C>
        + ObjDeptView<PageImage, C>
        + ObjDeptView<TeamAvatar, C>
        + ObjDeptView<UserAvatar, C>
        + Sync,
{
    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::Run,
        &token.user_id,
        &instr.comic_id,
    )
    .await?;

    chapter_perm_complex::ensure_user_can_list_infos(&member_info)?;

    let spec = ChapterListSpec {
        comic_id: instr.comic_id,
        incl_opt: instr.incl_opt,
        offset: instr.offset,
        limit: instr.limit,
    };

    let chapter_infos = ListChapterInfos { spec: &spec }.run_on(repo).await?;

    let chapter_info_vals =
        chapter_info_views(repo, obj_dept, chapter_infos).await?;

    accept(chapter_info_vals)
}

/// Fetches a chapter by ID.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn get_info<C, R>(
    (repo,): (&R,),
    token: UserToken,
    id: String,
) -> BaseRest<ChapterInfoView>
where
    C: Context,
    R: ChapterRepo<C> + MemberRepo<C> + TeamRepo<C> + Sync,
{
    let member_info = MemberLoader::load_info_from_chapter(
        repo,
        LoadMode::Run,
        &token.user_id,
        &id,
    )
    .await?;

    chapter_perm_complex::ensure_user_can_get_info(&member_info)?;

    let chapter_info = GetChapterInfo {
        id: &id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    accept(ChapterInfoView::from(chapter_info))
}

/// Fetches the pinned chapter under one comic.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn get_pinned<C, R>(
    (repo,): (&R,),
    token: UserToken,
    comic_id: String,
) -> BaseRest<Option<ChapterInfoView>>
where
    C: Context,
    R: ChapterRepo<C> + MemberRepo<C> + TeamRepo<C> + Sync,
{
    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::Run,
        &token.user_id,
        &comic_id,
    )
    .await?;

    chapter_perm_complex::ensure_user_can_get_pinned(&member_info)?;

    let chapter_info = FindPinnedChapterInfo {
        comic_id: &comic_id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    accept(chapter_info.map(ChapterInfoView::from))
}

/// Creates a new chapter.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: CreateChapterInstr,
) -> BaseRest<CreateChapterVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + AssignmentRepo<C>
        + Send
        + Sync,
{
    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::Run,
        &token.user_id,
        &instr.comic_id,
    )
    .await?;

    chapter_perm_complex::ensure_user_can_create(
        &member_info,
        instr.preset_assignment_roles,
    )?;

    let chapter_id = nucl
        .coord(async move |context| {
            //
            let comic_info = GetComicInfoExcluded {
                id: &instr.comic_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            comic_complex::ensure_comic_writable(&comic_info)?;

            LockChapters {
                comic_id: &instr.comic_id,
            }
            .step_on(repo, context)
            .await?;

            let prev_pinned_chapter = FindPinnedChapterInfo {
                comic_id: &instr.comic_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            let chapter_info = chapter_creation_usecase::create(
                repo,
                context,
                &comic_info,
                prev_pinned_chapter,
                &token,
                instr.subtitle,
                instr.preset_assignment_roles,
            )
            .await?;

            accept(chapter_info.id)
        })
        .await?;

    accept(CreateChapterVal { id: chapter_id })
}

/// Updates chapter metadata.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn update_info<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: UpdateChapterInfoInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + Send
        + Sync,
{
    let member_info = MemberLoader::load_info_from_chapter(
        repo,
        LoadMode::Run,
        &token.user_id,
        &instr.id,
    )
    .await?;

    chapter_perm_complex::ensure_user_can_update_info(&member_info)?;

    nucl.coord(async move |context| {
        //
        let chapter_info = GetChapterInfoExcluded {
            id: &instr.id,
            incls: &[],
        }
        .step_on(repo, context)
        .await?;

        chapter_complex::ensure_chapter_writable(&chapter_info)?;

        match instr.subtitle {
            //
            Some(next_subtitle) if next_subtitle != chapter_info.subtitle => {
                //
                let chapter_info_update = ChapterPatch {
                    id: instr.id,
                    subtitle: Some(next_subtitle.clone()),
                    pin: None,
                };

                UpdateChapter {
                    update: &chapter_info_update,
                }
                .step_on(repo, context)
                .await?;

                let workflow_record_entry = ChapterWorkflowRecordEntry::new(
                    chapter_info.id.clone(),
                    Some(token.user_id.clone()),
                    ChapterWorkflowRecordPayload::ChapterSubtitleUpdated {
                        previous_subtitle: chapter_info.subtitle,
                        next_subtitle,
                    },
                );

                CreateChapterWorkflowRecords {
                    entries: std::slice::from_ref(&workflow_record_entry),
                }
                .step_on(repo, context)
                .await?;
            }

            Some(_) | None => {}
        }

        TouchComicLastActive {
            id: &chapter_info.comic_id,
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Marks a chapter as the pinned chapter for its comic.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn mark_pinned<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + Send
        + Sync,
{
    let member_info = MemberLoader::load_info_from_chapter(
        repo,
        LoadMode::Run,
        &token.user_id,
        &id,
    )
    .await?;

    chapter_perm_complex::ensure_user_can_mark_pinned(&member_info)?;

    let chapter_info = GetChapterInfo {
        id: &id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    let comic_id = chapter_info.comic_id;

    let () = nucl
        .coord(async move |context| {
            //
            LockChapters {
                comic_id: &comic_id,
            }
            .step_on(repo, context)
            .await?;

            let chapter_info = GetChapterInfoExcluded {
                id: &id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            chapter_complex::ensure_chapter_writable(&chapter_info)?;

            let prev_pinned_chapter = FindPinnedChapterInfo {
                comic_id: &chapter_info.comic_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            UnpinOtherChapters {
                comic_id: &chapter_info.comic_id,
                excluded_id: &chapter_info.id,
            }
            .step_on(repo, context)
            .await?;

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

            let mut workflow_record_entries = Vec::with_capacity(2);

            if let Some(prev_pinned_chapter) = prev_pinned_chapter
                && prev_pinned_chapter.id != chapter_info.id
            {
                //
                workflow_record_entries.push(ChapterWorkflowRecordEntry::new(
                    prev_pinned_chapter.id,
                    Some(token.user_id.clone()),
                    ChapterWorkflowRecordPayload::ChapterUnpinned,
                ));
            }

            if !chapter_info.is_pinned {
                //
                workflow_record_entries.push(ChapterWorkflowRecordEntry::new(
                    chapter_info.id.clone(),
                    Some(token.user_id.clone()),
                    ChapterWorkflowRecordPayload::ChapterPinned,
                ));
            }

            CreateChapterWorkflowRecords {
                entries: &workflow_record_entries,
            }
            .step_on(repo, context)
            .await?;

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            accept(())
        })
        .await?;

    accept(())
}
