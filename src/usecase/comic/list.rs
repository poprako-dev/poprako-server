use std::collections::HashMap;

use poprako_orchestra::{Context, OperRun as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::comic::{ComicComplex, ComicPermComplex};
use crate::data::instr::comic::ListComicInfosInstr;
use crate::data::val::comic_list::ListComicInfosVal;
use crate::data::view::assignment::AssignmentInfoView;
use crate::data::view::chapter::ChapterInfoView;
use crate::data::view::comic::ComicInfoView;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::shared::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::ListAssignmentInfos;
use crate::part::repo::oper::chapter::ListPinnedChapterInfos;
use crate::part::repo::oper::comic::ListComicInfos;
use crate::part::repo::page::PageRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::page::PageLoader;
use crate::usecase::internal::util::{LoadMode, collect_bounded};
use crate::value::assignment::AssignmentInclOpt;
use crate::value::comic::ComicWithOpt;

/// Lists comics for a workset with optional filters and derived data.
#[instrument(level = "info", skip(repo, image_pool))]
pub async fn list_infos<C, R, I>(
    (repo, image_pool): (&R, &I),
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
    I: ImagePool + Sync,
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

    ComicPermComplex::ensure_user_can_list_infos(&member_info)?;

    let spec = instr.try_into()?;

    let comic_infos = ListComicInfos { spec: &spec }.run_on(repo).await?;

    let comic_ids = comic_infos
        .iter()
        .map(|comic_info| comic_info.id.clone())
        .collect::<Vec<_>>();

    let first_page_infos =
        PageLoader::load_infos_from_comics(repo, &comic_ids).await?;

    let fallback_cover_keys =
        ComicComplex::resolve_fallback_cover_keys(first_page_infos);

    // NOTE: `with` cannot be executed elegantly by repo layer,
    // so we have to handle it in usecase layer.
    let pinned_chapter_infos = if with_pinned_chapter {
        //
        ListPinnedChapterInfos {
            comic_ids: &comic_ids,
        }
        .run_on(repo)
        .await?
        .into_iter()
        .map(|chapter_info| (chapter_info.comic_id.clone(), chapter_info))
        .collect::<HashMap<_, _>>()
    } else {
        HashMap::new()
    };

    let pinned_chapter_assignment_infos = if with_pinned_chapter_assignment {
        //
        let chapter_ids = pinned_chapter_infos
            .values()
            .map(|chapter_info| chapter_info.id.clone())
            .collect::<Vec<_>>();

        let assignment_incls = [AssignmentInclOpt::User];

        let assignment_infos = ListAssignmentInfos::Chapters {
            chapter_ids: &chapter_ids,
            incls: &assignment_incls,
        }
        .run_on(repo)
        .await?;

        let mut assignment_infos_by_chapter = HashMap::new();

        for assignment_info in assignment_infos {
            //
            assignment_infos_by_chapter
                .entry(assignment_info.chapter_id.clone())
                .or_insert_with(Vec::new)
                .push(assignment_info);
        }

        assignment_infos_by_chapter
    } else {
        HashMap::new()
    };

    let assignment_view_pairs = collect_bounded(
        pinned_chapter_assignment_infos.into_values().flatten().map(
            |assignment_info| async move {
                //
                let chapter_id = assignment_info.chapter_id.clone();

                let assignment_view = AssignmentInfoView::from_model(
                    image_pool,
                    assignment_info,
                    None,
                )
                .await?;

                accept((chapter_id, assignment_view))
            },
        ),
    )
    .await?;

    let mut assignment_views_by_chapter = HashMap::new();

    for (chapter_id, assignment_view) in assignment_view_pairs {
        //
        assignment_views_by_chapter
            .entry(chapter_id)
            .or_insert_with(Vec::new)
            .push(assignment_view);
    }

    build_list_val(
        image_pool,
        comic_infos,
        pinned_chapter_infos,
        assignment_views_by_chapter,
        fallback_cover_keys,
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

// Build aligned comic, pinned-chapter, and assignment response vectors.
async fn build_list_val<I>(
    image_pool: &I,
    comic_infos: Vec<ComicInfo>,
    mut pinned_chapter_infos: HashMap<String, ChapterInfo>,
    mut assignment_views_by_chapter: HashMap<String, Vec<AssignmentInfoView>>,
    fallback_cover_keys: HashMap<String, String>,
) -> BaseRest<ListComicInfosVal>
where
    I: ImagePool + Sync,
{
    let conversion_inputs = comic_infos
        .into_iter()
        .map(|comic_info| {
            //
            let chapter_info = pinned_chapter_infos.remove(&comic_info.id);

            let assignment_views = chapter_info
                .as_ref()
                .and_then(|chapter_info| {
                    assignment_views_by_chapter.remove(&chapter_info.id)
                })
                .unwrap_or_default();

            let fallback_cover_key =
                fallback_cover_keys.get(&comic_info.id).cloned();

            (
                comic_info,
                chapter_info,
                assignment_views,
                fallback_cover_key,
            )
        })
        .collect::<Vec<_>>();

    let converted_infos =
        collect_bounded(conversion_inputs.into_iter().map(
            |(
                comic_info,
                chapter_info,
                assignment_views,
                fallback_cover_key,
            )| async move {
                //
                let chapter_view = match chapter_info {
                    //
                    Some(chapter_info) => Some(
                        ChapterInfoView::from_model(
                            image_pool,
                            chapter_info,
                            None,
                        )
                        .await?,
                    ),

                    None => None,
                };

                let comic_view = ComicInfoView::from_model(
                    image_pool,
                    comic_info,
                    fallback_cover_key.as_deref(),
                )
                .await?;

                accept((comic_view, chapter_view, assignment_views))
            },
        ))
        .await?;

    let mut comic_info_views = Vec::with_capacity(converted_infos.len());

    let mut pinned_chapter_views = Vec::with_capacity(converted_infos.len());

    let mut pinned_chapter_assignment_views =
        Vec::with_capacity(converted_infos.len());

    for (comic_info_view, pinned_chapter_view, assignment_views) in
        converted_infos
    {
        comic_info_views.push(comic_info_view);

        pinned_chapter_views.push(pinned_chapter_view);

        pinned_chapter_assignment_views.push(assignment_views);
    }

    accept(ListComicInfosVal {
        comics: comic_info_views,
        pinned_chapters: pinned_chapter_views,
        pinned_chapter_assignments: pinned_chapter_assignment_views,
    })
}
