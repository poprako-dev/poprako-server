use std::collections::HashMap;

use poprako_orchestra::run_proxy;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::comic::{ComicComplex, ComicPermComplex};
use crate::data::assignment::AssignmentInfoVal;
use crate::data::chapter::ChapterInfoVal;
use crate::data::comic::{ComicInfoVal, ListComicInfosParams};
use crate::data::comic_list::ListComicInfosPayload;
use crate::model::comic::ComicInfoListSpec;
use crate::model::user::UserToken;
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
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::value::comic::ComicWithOpt;

/// Lists comics for a workset with optional filters and derived data.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    params: ListComicInfosParams,
) -> BaseResult<ListComicInfosPayload>
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
    let with_pinned_chapter =
        params.with_opt.contains(&ComicWithOpt::PinnedChapter);

    let with_pinned_chapter_assignment = params
        .with_opt
        .contains(&ComicWithOpt::PinnedChapterAssignment);

    if with_pinned_chapter_assignment && !with_pinned_chapter {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-pinned-chapter-with-required"),
        });
    }

    ComicPermComplex::ensure_user_can_list_infos(
        &mut run_proxy! {
            repo =>
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &params.workset_id,
    )
    .await?;

    let spec: ComicInfoListSpec = params.try_into()?;

    let comic_infos = repo.run(&ListComicInfos { spec: &spec }).await?;

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
            repo.run(&ListPinnedChapterInfos {
                comic_ids: &comic_ids,
            })
            .await?
        }

        false => HashMap::new(),
    };

    let mut pinned_chapter_assignment_infos =
        match with_pinned_chapter_assignment {
            //
            true => {
                let chapter_ids = pinned_chapter_infos
                    .values()
                    .map(|chapter_info| chapter_info.id.clone())
                    .collect::<Vec<_>>();

                let assignment_infos = repo
                    .run(&ListAssignmentInfos::Chapters {
                        chapter_ids: &chapter_ids,
                        incls: &[],
                    })
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

    let mut comic_info_vals = Vec::with_capacity(comic_infos.len());

    let mut pinned_chapter_vals = Vec::with_capacity(comic_infos.len());

    let mut pinned_chapter_assignment_vals =
        Vec::with_capacity(comic_infos.len());

    for comic_info in comic_infos {
        let (pinned_chapter_val, pinned_chapter_assignment_val) =
            match pinned_chapter_infos.remove(&comic_info.id) {
                //
                Some(chapter_info) => {
                    let assignment_vals = pinned_chapter_assignment_infos
                        .remove(&chapter_info.id)
                        .unwrap_or_default()
                        .into_iter()
                        .map(AssignmentInfoVal::from)
                        .collect();

                    let chapter_val = ChapterInfoVal::from_model(
                        image_pool,
                        chapter_info,
                        None,
                    )
                    .await?;

                    (Some(chapter_val), assignment_vals)
                }

                None => (None, Vec::new()),
            };

        let fallback_cover_key =
            fallback_cover_keys.get(&comic_info.id).map(String::as_str);

        comic_info_vals.push(
            ComicInfoVal::from_model(
                image_pool,
                comic_info,
                fallback_cover_key,
            )
            .await?,
        );

        pinned_chapter_vals.push(pinned_chapter_val);

        pinned_chapter_assignment_vals.push(pinned_chapter_assignment_val);
    }

    accept(ListComicInfosPayload {
        comics: comic_info_vals,
        pinned_chapters: pinned_chapter_vals,
        pinned_chapter_assignments: pinned_chapter_assignment_vals,
    })
}
