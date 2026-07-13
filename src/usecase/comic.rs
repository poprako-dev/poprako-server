//! Comic use cases — create, read, update, cover management, and deletion.

use std::collections::HashMap;
use std::time::Duration;

use poprako_orchestra::{Nucl, run_proxy};
use poprako_orchestra_extra::prom::oper::Defer;
use poprako_orchestra_extra::prom::task::Task;

use crate::complex::assignment::AssignmentComplex;
use crate::complex::chapter::ChapterComplex;
use crate::complex::comic::{ComicComplex, ComicPermComplex};
use crate::complex::image::ImageComplex;
use crate::data::chapter::ChapterInfoVal;
use crate::data::comic::{
    ComicInfoVal, CreateComicParams, CreateComicPayload, ListComicInfosParams,
    MarkComicCoverUploadedParams, ReserveComicCoverParams,
    ReserveComicCoverPayload, UpdateComicInfoParams,
};
use crate::model::assignment::AssignmentEntry;
use crate::model::chapter::ChapterEntry;
use crate::model::comic::{ComicEntry, ComicInfoListSpec, ComicInfoUpdate};
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::prom::Prom;
use crate::part::prom::payload::{Payload, image};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::CreateAssignment;
use crate::part::repo::oper::chapter::{
    CreateChapter, ListPinnedChapterInfos, UnpinOtherChapters,
};
use crate::part::repo::oper::comic::{
    AllocateComicChapterIndex, CreateComic, GetComicInfo, ListComicInfos,
    MarkComicCoverUploaded, ReserveComicCover, TouchComicLastActive,
    UpdateComic, UpdateComicChapterCount,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::workset::{
    AllocateWorksetComicIndex, GetWorksetInfo, UpdateWorksetComicCount,
};
use crate::part::repo::page::PageRepo;
use crate::part::repo::unit::UnitRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{RegularError, RegularResult, accept};
use crate::value::comic::ComicWithOpt;
use crate::value::role::{RoleField, RoleMask};

#[cfg(test)]
pub mod tests;

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
pub async fn create<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: CreateComicParams,
) -> RegularResult<CreateComicPayload>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + AssignmentRepo<C>
        + Send
        + Sync,
{
    ComicPermComplex::ensure_user_can_create(
        &mut run_proxy! {
            repo =>
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &params.workset_id,
    )
    .await?;

    let (comic_id, chapter_id) = nucl
        .coord(async move |context| -> RegularResult<(String, String)> {
            //
            let index = repo
                .step(
                    context,
                    &AllocateWorksetComicIndex {
                        id: &params.workset_id,
                    },
                )
                .await?;

            let comic_entry = ComicEntry {
                id: ComicComplex::gen_id(),
                workset_id: params.workset_id,
                index,
                title: params.title,
                author: params.author,
                description: params.description,
                creator_id: token.user_id.clone(),
            };

            let comic_info = repo
                .step(
                    context,
                    &CreateComic {
                        entry: &comic_entry,
                    },
                )
                .await?;

            repo.step(
                context,
                &UpdateWorksetComicCount {
                    id: &comic_entry.workset_id,
                    delta: 1,
                },
            )
            .await?;

            let chapter_index = repo
                .step(
                    context,
                    &AllocateComicChapterIndex { id: &comic_info.id },
                )
                .await?;

            let subtitle = ChapterComplex::subtitle_or_default(
                params.first_chapter_subtitle,
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

            let chapter_info = repo
                .step(
                    context,
                    &CreateChapter {
                        entry: &chapter_entry,
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

            repo.step(
                context,
                &UpdateComicChapterCount {
                    id: &chapter_info.comic_id,
                    delta: 1,
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

            let assignment_entry = AssignmentEntry {
                id: AssignmentComplex::gen_id(),
                chapter_id: chapter_info.id.clone(),
                user_id: token.user_id,
                roles: RoleMask::from(RoleField::ADMIN),
            };

            repo.step(
                context,
                &CreateAssignment {
                    entry: &assignment_entry,
                },
            )
            .await?;

            Ok((comic_info.id, chapter_info.id))
        })
        .await?;

    Ok(CreateComicPayload {
        id: comic_id,
        chapter_id,
    })
}

/// Fetches a comic by ID with cover URL resolution.
pub async fn get_info<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    id: String,
) -> RegularResult<ComicInfoVal>
where
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
    I: ImagePool,
{
    ComicPermComplex::ensure_user_can_get_info(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &id,
    )
    .await?;

    let comic_info = repo
        .run(&GetComicInfo {
            id: &id,
            incls: &[],
        })
        .await?;

    ComicInfoVal::from_model(image_pool, comic_info, None).await
}

/// Lists comics for a workset with optional title filter, completion filter, and pagination.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    params: ListComicInfosParams,
) -> RegularResult<Vec<ComicInfoVal>>
where
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + ChapterRepo<C> + Sync,
    I: ImagePool,
{
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

    let with_pinned_chapter =
        params.with_opt.contains(&ComicWithOpt::PinnedChapter);

    let spec: ComicInfoListSpec = params.try_into()?;

    let comic_infos = repo.run(&ListComicInfos { spec: &spec }).await?;

    // NOTE: `with` cannot be executed elegantly by repo layer,
    // so we have to handle it in usecase layer.
    let mut pinned_chapters = match with_pinned_chapter {
        //
        true => {
            //
            let comic_ids: Vec<String> =
                comic_infos.iter().map(|info| info.id.clone()).collect();

            repo.run(&ListPinnedChapterInfos {
                comic_ids: &comic_ids,
            })
            .await?
        }

        false => HashMap::new(),
    };

    let mut comic_info_vals = Vec::with_capacity(comic_infos.len());

    for comic_info in comic_infos {
        //
        let pinned_chapter_val = match pinned_chapters.remove(&comic_info.id) {
            //
            Some(chapter_info) => Some(
                ChapterInfoVal::from_model(image_pool, chapter_info).await?,
            ),

            None => None,
        };

        comic_info_vals.push(
            ComicInfoVal::from_model(
                image_pool,
                comic_info,
                pinned_chapter_val,
            )
            .await?,
        );
    }

    Ok(comic_info_vals)
}

/// Updates a comic's title, author, and description.
pub async fn update_info<C, R>(
    repo: &R,
    token: UserToken,
    params: UpdateComicInfoParams,
) -> RegularResult<()>
where
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
{
    ComicPermComplex::ensure_user_can_update_info(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &params.id,
    )
    .await?;

    let comic_info_update = ComicInfoUpdate {
        id: params.id,
        title: params.title,
        author: params.author,
        description: params.description,
    };

    repo.run(&UpdateComic {
        update: &comic_info_update,
    })
    .await?;

    accept(())
}

/// Reserves a new comic cover upload slot.
pub async fn reserve_cover<N, C, R, P, I>(
    nucl: &N,
    repo: &R,
    prom: &P,
    image_pool: &I,
    token: UserToken,
    id: String,
    params: ReserveComicCoverParams,
) -> RegularResult<ReserveComicCoverPayload>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    ComicPermComplex::ensure_user_can_reserve_cover(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &id,
    )
    .await?;

    let (object_key, cover_version) = nucl
        .coord(async move |context| -> RegularResult<(String, u32)> {
            //
            let cover_reservation = repo
                .step(
                    context,
                    &ReserveComicCover {
                        id: &id,
                        file_extension: &params.file_ext,
                    },
                )
                .await?;

            if let Some(prev_object_key) = &cover_reservation.prev_object_key {
                //
                let delete_id = ImageComplex::gen_delete_id();

                let delete_payload = Payload::Image(image::Payload::Delete {
                    object_key: prev_object_key.clone(),
                });

                let delete_task = Task {
                    id: &delete_id,
                    payload: &delete_payload,
                    delay: None,
                };

                prom.step(context, &Defer::new(delete_task)).await?;
            }

            let check_id = ImageComplex::gen_check_id();

            let check_payload = Payload::Image(image::Payload::CheckUpload {
                resource_kind: image::ResourceKind::ComicCover,
                resource_id: id.clone(),
                object_key: cover_reservation.object_key.clone(),
                version: cover_reservation.cover_version,
            });

            let check_task = Task {
                id: &check_id,
                payload: &check_payload,
                delay: Some(Duration::from_secs(15 * 60)),
            };

            prom.step(context, &Defer::new(check_task)).await?;

            Ok((
                cover_reservation.object_key,
                cover_reservation.cover_version,
            ))
        })
        .await?;

    let put_url = image_pool.put_signed(&object_key).await?.to_string();

    Ok(ReserveComicCoverPayload {
        put_url,
        cover_version,
    })
}

/// Marks a reserved comic cover as successfully uploaded.
pub async fn mark_cover_uploaded<C, R>(
    repo: &R,
    token: UserToken,
    id: String,
    params: MarkComicCoverUploadedParams,
) -> RegularResult<()>
where
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
{
    ComicPermComplex::ensure_user_can_mark_cover_uploaded(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &id,
    )
    .await?;

    repo.run(&MarkComicCoverUploaded {
        id: &id,
        cover_version: params.cover_version,
    })
    .await?;

    accept(())
}

/// Deletes a comic and updates the parent workset counter.
pub async fn delete<N, C, R, P>(
    nucl: &N,
    repo: &R,
    prom: &P,
    token: UserToken,
    id: String,
) -> RegularResult<()>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + PageRepo<C>
        + AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    ComicPermComplex::ensure_user_can_delete(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &id,
    )
    .await?;

    nucl.coord(async move |context| -> RegularResult<()> {
        //
        ComicComplex::delete_cascade(repo, prom, context, &id).await?;

        accept(())
    })
    .await?;

    accept(())
}
