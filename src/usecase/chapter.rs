//! Chapter use cases — list, read, create, update, and deletion.

/// Chapter deletion use cases.
pub mod delete;
/// Chapter workflow stage mutation use case.
pub mod stage;
/// Immutable workflow record listing use case.
pub mod workflow_record;

#[cfg(test)]
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::assignment::AssignmentComplex;
use crate::complex::chapter::{ChapterComplex, ChapterPermComplex};
use crate::complex::comic::ComicComplex;
use crate::data::instr::chapter::{
    CreateChapterInstr, ListChapterInfosInstr, UpdateChapterInfoInstr,
};
use crate::data::val::chapter::CreateChapterVal;
use crate::data::view::chapter::ChapterInfoView;
use crate::model::read::spec::chapter::ChapterListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::assignment::AssignmentEntry;
use crate::model::write::chapter::{ChapterEntry, ChapterPatch};
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::image::ImagePool;
use crate::part::nucl::ReptRead;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::{
    CreateAssignment, FindAssignmentInfo,
};
use crate::part::repo::oper::chapter::{
    CreateChapter, FindPinnedChapterInfo, GetChapterInfo,
    GetChapterInfoExcluded, ListChapterInfos, LockChapters, UnpinOtherChapters,
    UpdateChapter,
};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic::{
    AllocComicChapterIndex, GetComicInfoExcluded, TouchComicLastActive,
    UpdateComicChapterCount,
};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::page::PageLoader;
use crate::usecase::internal::util::LoadMode;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;

/// Lists chapters under one comic.
#[instrument(level = "info", skip(repo, image_pool))]
pub async fn list_infos<C, R, I>(
    (repo, image_pool): (&R, &I),
    token: UserToken,
    instr: ListChapterInfosInstr,
) -> BaseRest<Vec<ChapterInfoView>>
where
    C: Context,
    R: ChapterRepo<C> + MemberRepo<C> + TeamRepo<C> + PageRepo<C> + Sync,
    I: ImagePool,
{
    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::Run,
        &token.user_id,
        &instr.comic_id,
    )
    .await?;

    ChapterPermComplex::ensure_user_can_list_infos(&member_info)?;

    let spec = ChapterListSpec {
        comic_id: instr.comic_id,
        incl_opt: instr.incl_opt,
        offset: instr.offset,
        limit: instr.limit,
    };

    let chapter_infos = ListChapterInfos { spec: &spec }.run_on(repo).await?;

    let comic_ids = chapter_infos
        .iter()
        .filter_map(|chapter_info| chapter_info.comic.as_ref())
        .map(|comic_info| comic_info.id.clone())
        .collect::<Vec<_>>();

    let first_page_infos =
        PageLoader::load_infos_from_comics(repo, &comic_ids).await?;

    let fallback_cover_keys =
        ComicComplex::resolve_fallback_cover_keys(first_page_infos);

    let mut chapter_info_vals = Vec::with_capacity(chapter_infos.len());

    for chapter_info in chapter_infos {
        //
        let fallback_cover_key = chapter_info
            .comic
            .as_ref()
            .and_then(|comic_info| fallback_cover_keys.get(&comic_info.id))
            .map(String::as_str);

        chapter_info_vals.push(
            ChapterInfoView::from_model(
                image_pool,
                chapter_info,
                fallback_cover_key,
            )
            .await?,
        );
    }

    accept(chapter_info_vals)
}

/// Fetches a chapter by ID.
#[instrument(level = "info", skip(repo))]
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

    ChapterPermComplex::ensure_user_can_get_info(&member_info)?;

    let chapter_info = GetChapterInfo {
        id: &id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    accept(ChapterInfoView::from(chapter_info))
}

/// Fetches the pinned chapter under one comic.
#[instrument(level = "info", skip(repo))]
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

    ChapterPermComplex::ensure_user_can_get_pinned(&member_info)?;

    let chapter_info = FindPinnedChapterInfo {
        comic_id: &comic_id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    accept(chapter_info.map(ChapterInfoView::from))
}

/// Creates a new chapter.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: CreateChapterInstr,
) -> BaseRest<CreateChapterVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
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

    ChapterPermComplex::ensure_user_can_create(
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

            ComicComplex::ensure_comic_writable(&comic_info)?;

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

            let index = AllocComicChapterIndex {
                id: &instr.comic_id,
            }
            .step_on(repo, context)
            .await?;

            let subtitle =
                ChapterComplex::subtitle_or_default(instr.subtitle, index);

            let chapter_id = ChapterComplex::gen_id();

            UnpinOtherChapters {
                comic_id: &instr.comic_id,
                excluded_id: &chapter_id,
            }
            .step_on(repo, context)
            .await?;

            let chapter_entry = ChapterEntry {
                id: chapter_id,
                comic_id: instr.comic_id,
                is_pinned: true,
                index,
                subtitle,
                creator_id: token.user_id.clone(),
            };

            let chapter_info = CreateChapter {
                entry: &chapter_entry,
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

            let mut workflow_record_entries = Vec::with_capacity(2);

            if let Some(prev_pinned_chapter) = prev_pinned_chapter {
                //
                workflow_record_entries.push(ChapterWorkflowRecordEntry::new(
                    prev_pinned_chapter.id,
                    Some(token.user_id.clone()),
                    ChapterWorkflowRecordPayload::ChapterUnpinned,
                ));
            }

            workflow_record_entries.push(ChapterWorkflowRecordEntry::new(
                chapter_info.id.clone(),
                Some(token.user_id),
                ChapterWorkflowRecordPayload::ChapterCreated,
            ));

            CreateChapterWorkflowRecords {
                entries: &workflow_record_entries,
            }
            .step_on(repo, context)
            .await?;

            accept(chapter_info.id)
        })
        .await?;

    accept(CreateChapterVal { id: chapter_id })
}

/// Updates chapter metadata.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn update_info<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: UpdateChapterInfoInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + AssignmentRepo<C>
        + Send
        + Sync,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id: &instr.id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-chapter-admin-required"),
        });
    };

    ChapterPermComplex::ensure_user_can_update_info(&assignment_info)?;

    nucl.coord(async move |context| {
        //
        let chapter_info = GetChapterInfoExcluded {
            id: &instr.id,
            incls: &[],
        }
        .step_on(repo, context)
        .await?;

        ChapterComplex::ensure_chapter_writable(&chapter_info)?;

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
#[instrument(level = "info", skip(nucl, repo))]
pub async fn mark_pinned<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + AssignmentRepo<C>
        + Send
        + Sync,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id: &id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-chapter-admin-required"),
        });
    };

    ChapterPermComplex::ensure_user_can_mark_pinned(&assignment_info)?;

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

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

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
