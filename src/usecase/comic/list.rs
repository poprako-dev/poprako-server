use std::collections::HashMap;

use poprako_orchestra::{Context, OperRun as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;
use poprako_util::i18n::trl;

use crate::complex::comic::perm as comic_perm_complex;
use crate::data::instr::comic::ListComicInfosInstr;
use crate::data::val::comic_list::ListComicInfosVal;
use crate::model::shared::user::UserToken;
use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar, UserAvatar};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::ListAssignmentInfos;
use crate::part::repo::oper::comic::ListComicInfos;
use crate::part::repo::page::PageRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::page::PinnedChapterSnapshot;
use crate::usecase::internal::util::LoadMode;
use crate::usecase::internal::view::comic_list_val;
use crate::value::assignment::AssignmentInclOpt;
use crate::value::comic::ComicWithOpt;

/// Lists comics for a workset with optional filters and derived data.
#[instrument(level = "info", skip(repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn list_infos<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    instr: ListComicInfosInstr,
) -> BaseRest<ListComicInfosVal>
where
    C: Context,
    R: ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + Sync,
    O: ObjDeptView<ComicCover, C>
        + ObjDeptView<PageImage, C>
        + ObjDeptView<TeamAvatar, C>
        + ObjDeptView<UserAvatar, C>
        + Sync,
{
    let (with_pinned_chapter, with_pinned_chapter_assignment) = (
        instr.with_opt.contains(&ComicWithOpt::PinnedChapter),
        instr
            .with_opt
            .contains(&ComicWithOpt::PinnedChapterAssignment),
    );

    validate_with_options(
        &instr,
        &token.user_id,
        with_pinned_chapter,
        with_pinned_chapter_assignment,
    )?;

    let member_info = MemberLoader::load_info_from_workset(
        repo,
        LoadMode::Run,
        &token.user_id,
        &instr.workset_id,
    )
    .await?;

    comic_perm_complex::ensure_user_can_list_infos(&member_info)?;

    let spec = instr.try_into()?;

    let comic_infos = ListComicInfos { spec: &spec }.run_on(repo).await?;

    let comic_ids = comic_infos
        .iter()
        .map(|comic_info| comic_info.id.as_str())
        .collect::<Vec<_>>();

    // NOTE: `with` cannot be executed elegantly by repo layer,
    // so we have to handle it in usecase layer.
    let pinned_chapter_snapshot = match (with_pinned_chapter,) {
        //
        (true,) => Some(
            PinnedChapterSnapshot::load_from_comics(repo, &comic_ids).await?,
        ),

        (false,) => None,
    };

    let pinned_chapter_infos = pinned_chapter_snapshot
        .as_ref()
        .map(PinnedChapterSnapshot::infos_by_comic_id);

    let pinned_chapter_assignment_infos = match (with_pinned_chapter_assignment,)
    {
        (true,) => {
            //
            let chapter_ids = pinned_chapter_infos
                .into_iter()
                .flat_map(|pinned_chapter_infos| pinned_chapter_infos.values())
                .map(|chapter_info| chapter_info.id.as_str())
                .collect::<Vec<_>>();

            let assignment_incls = [AssignmentInclOpt::User];

            let assignment_infos = ListAssignmentInfos::Chapters {
                chapter_ids: &chapter_ids,
                incls: &assignment_incls,
            }
            .run_on(repo)
            .await?;

            let mut assignment_infos_by_chapter = HashMap::<_, Vec<_>>::new();

            for assignment_info in assignment_infos {
                //
                if let Some(chapter_assignments) = assignment_infos_by_chapter
                    .get_mut(&assignment_info.chapter_id)
                {
                    //
                    chapter_assignments.push(assignment_info);

                    continue;
                }

                assignment_infos_by_chapter.insert(
                    assignment_info.chapter_id.clone(),
                    vec![assignment_info],
                );
            }

            assignment_infos_by_chapter
        }

        (false,) => HashMap::new(),
    };

    comic_list_val(
        repo,
        obj_dept,
        comic_infos,
        pinned_chapter_snapshot,
        pinned_chapter_assignment_infos,
    )
    .await
}

// Validate dependencies between optional pinned-chapter response fields.
fn validate_with_options(
    instr: &ListComicInfosInstr,
    user_id: &str,
    with_pinned_chapter: bool,
    with_pinned_chapter_assignment: bool,
) -> BaseRest<()> {
    //
    if !with_pinned_chapter_assignment || with_pinned_chapter {
        return accept(());
    }

    let err_message = trl("error-pinned-chapter-with-required");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        workset_id = %instr.workset_id,
        user_id = %user_id,
        with_pinned_chapter,
        with_pinned_chapter_assignment,
        "expected error: pinned chapter assignment requires pinned chapter",
    );

    Err(BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    })
}
