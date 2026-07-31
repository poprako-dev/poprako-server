use std::collections::HashMap;

use poprako_orchestra::{OperRun as _, run_proxy};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::comic::{ComicComplex, ComicPermComplex};
use crate::data::instr::comic::ListComicInfosInstr;
use crate::data::val::comic_list::ListComicInfosVal;
use crate::data::view::assignment::AssignmentInfoView;
use crate::data::view::chapter::ChapterInfoView;
use crate::data::view::comic::ComicInfoView;
use crate::model::read::spec::comic::ComicListSpec;
use crate::model::shared::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::ListAssignmentInfos;
use crate::part::repo::oper::chapter::ListPinnedChapterInfos;
use crate::part::repo::oper::comic::ListComicInfos;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::page::ListFirstPageInfos;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::page::PageRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::comic::ComicWithOpt;

/// Lists comics for a workset with optional filters and derived data.
#[instrument(level = "info", err(Debug), skip(repo, image_pool))]
pub async fn list_infos<C, R, I>(
    (repo, image_pool): (&R, &I),
    token: UserToken,
    instr: ListComicInfosInstr,
) -> BaseRest<ListComicInfosVal>
where
    R: ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + Sync,
    I: ImagePool,
{
    let (with_pinned_chapter, with_pinned_chapter_assignment) = (
        instr.with_opt.contains(&ComicWithOpt::PinnedChapter),
        instr
            .with_opt
            .contains(&ComicWithOpt::PinnedChapterAssignment),
    );

    if with_pinned_chapter_assignment && !with_pinned_chapter {
        //
        let err_message = trl("error-pinned-chapter-with-required");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            workset_id = %instr.workset_id,
            user_id = %token.user_id,
            with_pinned_chapter,
            with_pinned_chapter_assignment,
            "expected error: pinned chapter assignment requires pinned chapter",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    ComicPermComplex::ensure_user_can_list_infos(
        &mut run_proxy! {
            repo =>
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &instr.workset_id,
    )
    .await?;

    let spec: ComicListSpec = instr.try_into()?;

    let comic_infos = ListComicInfos { spec: &spec }.run_on(repo).await?;

    let comic_ids = comic_infos
        .iter()
        .map(|comic_info| comic_info.id.clone())
        .collect::<Vec<_>>();

    let fallback_cover_keys = ComicComplex::resolve_fallback_cover_keys(
        &mut run_proxy! {
            repo =>
                for<'a> ListPinnedChapterInfos<'a>,
                for<'a> ListFirstPageInfos<'a>;
        },
        &comic_ids,
    )
    .await?;

    // NOTE: `with` cannot be executed elegantly by repo layer,
    // so we have to handle it in usecase layer.
    let mut pinned_chapter_infos = match with_pinned_chapter {
        //
        true => {
            ListPinnedChapterInfos {
                comic_ids: &comic_ids,
            }
            .run_on(repo)
            .await?
        }

        false => HashMap::new(),
    };

    let mut pinned_chapter_assignment_infos =
        match with_pinned_chapter_assignment {
            //
            true => {
                //
                let chapter_ids = pinned_chapter_infos
                    .values()
                    .map(|chapter_info| chapter_info.id.clone())
                    .collect::<Vec<_>>();

                let assignment_infos = ListAssignmentInfos::Chapters {
                    chapter_ids: &chapter_ids,
                    incls: &[],
                }
                .run_on(repo)
                .await?;

                let mut assignment_infos_by_chapter = HashMap::new();

                for assignment_info in assignment_infos {
                    assignment_infos_by_chapter
                        .entry(assignment_info.chapter_id.clone())
                        .or_insert_with(Vec::new)
                        .push(assignment_info);
                }

                assignment_infos_by_chapter
            }

            false => HashMap::new(),
        };

    let (
        mut comic_info_views,
        mut pinned_chapter_views,
        mut pinned_chapter_assignment_views,
    ) = (
        Vec::with_capacity(comic_infos.len()),
        Vec::with_capacity(comic_infos.len()),
        Vec::with_capacity(comic_infos.len()),
    );

    for comic_info in comic_infos {
        //
        let (pinned_chapter_view, assignment_views_for_chapter) =
            match pinned_chapter_infos.remove(&comic_info.id) {
                //
                Some(chapter_info) => {
                    //
                    let assignment_views = pinned_chapter_assignment_infos
                        .remove(&chapter_info.id)
                        .unwrap_or_default()
                        .into_iter()
                        .map(AssignmentInfoView::from)
                        .collect();

                    let chapter_view = ChapterInfoView::from_model(
                        image_pool,
                        chapter_info,
                        None,
                    )
                    .await?;

                    (Some(chapter_view), assignment_views)
                }

                None => (None, Vec::new()),
            };

        let fallback_cover_key =
            fallback_cover_keys.get(&comic_info.id).map(String::as_str);

        comic_info_views.push(
            ComicInfoView::from_model(
                image_pool,
                comic_info,
                fallback_cover_key,
            )
            .await?,
        );

        pinned_chapter_views.push(pinned_chapter_view);

        pinned_chapter_assignment_views.push(assignment_views_for_chapter);
    }

    accept(ListComicInfosVal {
        comics: comic_info_views,
        pinned_chapters: pinned_chapter_views,
        pinned_chapter_assignments: pinned_chapter_assignment_views,
    })
}
