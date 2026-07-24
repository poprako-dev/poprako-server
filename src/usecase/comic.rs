//! Comic use cases — create, read, update, cover management, and deletion.

use std::time::Duration;

use poprako_orchestra::{Nucl, run_proxy, step_proxy};
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use poprako_orchestra_extra::prom::task::Task;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::assignment::AssignmentComplex;
use crate::complex::chapter::ChapterComplex;
use crate::complex::comic::{ComicComplex, ComicPermComplex};
use crate::complex::image::ImageComplex;
use crate::data::comic::{ComicInfoVal, CreateComicParams, CreateComicPayload, MarkComicCoverUploadedParams, ReserveComicCoverParams, ReserveComicCoverPayload, UpdateComicInfoParams};
use crate::data::image::ImageUploadSlotVal;
use crate::model::assignment::AssignmentEntry;
use crate::model::chapter::ChapterEntry;
use crate::model::comic::{ComicEntry, ComicInfoUpdate};
use crate::model::user::UserToken;
use crate::part::image::{ImageManager, ImagePool, ImageUploadSpec};
use crate::part::prom::Prom;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::{CreateAssignment, DeleteAssignments};
use crate::part::repo::oper::assignment_invitation::DeleteAssignmentInvitations;
use crate::part::repo::oper::chapter::{CreateChapter, DeleteChapter, GetChapterInfoExcluded, ListChapterInfosExcluded, ListPinnedChapterInfos, UnpinOtherChapters, UpdateChapter};
use crate::part::repo::oper::comic::{AllocComicChapterIndex, CreateComic, DeleteComic, GetComicInfo, GetComicInfoExcluded, MarkComicCoverUploaded, ReserveComicCover, TouchComicLastActive, UpdateComic, UpdateComicChapterCount};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::page::{DeletePages, ListFirstPageInfos, ListPageInfos};
use crate::part::repo::oper::term::DeleteTerms;
use crate::part::repo::oper::termbase::{DeleteTermbase, GetTermbaseInfoExcluded, ListTermbaseInfosExcluded};
use crate::part::repo::oper::workset::{AllocWorksetComicIndex, GetWorksetInfo, UpdateWorksetComicCount};
use crate::part::repo::page::PageRepo;
use crate::part::repo::term::TermRepo;
use crate::part::repo::termbase::TermbaseRepo;
use crate::part::repo::unit::UnitRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};

pub use list::list_infos;

mod list;
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
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn create<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: CreateComicParams,
) -> BaseResult<CreateComicPayload>
where
    N: Nucl<Context = C, Error = BaseError>,
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
        params.preset_assignment_roles,
    )
    .await?;

    let (comic_id, chapter_id) = nucl
        .coord(async move |context| {
            //
            let index = repo
                .step(
                    context,
                    &AllocWorksetComicIndex {
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
                .step(context, &AllocComicChapterIndex { id: &comic_info.id })
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
                roles: AssignmentComplex::creator_roles(
                    params.preset_assignment_roles,
                ),
            };

            repo.step(
                context,
                &CreateAssignment {
                    entry: &assignment_entry,
                },
            )
            .await?;

            accept((comic_info.id, chapter_info.id))
        })
        .await?;

    accept(CreateComicPayload {
        id: comic_id,
        chapter_id,
    })
}

/// Fetches a comic by ID with cover URL resolution.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn get_info<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    id: String,
) -> BaseResult<ComicInfoVal>
where
    R: ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + PageRepo<C>
        + Sync,
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

    let comic_ids = vec![comic_info.id.clone()];

    let fallback_cover_keys = ComicComplex::resolve_fallback_cover_keys(
        &mut run_proxy! {
            repo =>
                for<'a> ListPinnedChapterInfos<'a>,
                for<'a> ListFirstPageInfos<'a>;
        },
        &comic_ids,
    )
    .await?;

    ComicInfoVal::from_model(
        image_pool,
        comic_info,
        fallback_cover_keys.get(&id).map(String::as_str),
    )
    .await
}

/// Updates a comic's title, author, and description.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn update_info<C, R>(
    repo: &R,
    token: UserToken,
    params: UpdateComicInfoParams,
) -> BaseResult<()>
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
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn reserve_cover<N, C, R, P, I>(
    nucl: &N,
    repo: &R,
    prom: &P,
    image_pool: &I,
    token: UserToken,
    id: String,
    params: ReserveComicCoverParams,
) -> BaseResult<ReserveComicCoverPayload>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    ImageComplex::ensure_byte_length(
        params.byte_length,
        image::ResourceKind::ComicCover,
    )?;

    let image_hash = params.image_hash.clone();

    let transaction_image_hash = image_hash.clone();

    let image_ext = params.ext;

    let byte_length = params.byte_length;

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

    let (object_key, cover_version, upload_required) = nucl
        .coord(async move |context| {
            //
            let cover_reservation = repo
                .step(
                    context,
                    &ReserveComicCover {
                        id: &id,
                        image_hash: &transaction_image_hash,
                        image_ext,
                    },
                )
                .await?;

            if !cover_reservation.upload_required {
                return accept((
                    cover_reservation.object_key,
                    cover_reservation.cover_version,
                    false,
                ));
            }

            let mut batch_ids = Vec::new();

            let mut batch_payloads = Vec::new();

            let mut batch_delays = Vec::new();

            if let Some(prev_object_key) = &cover_reservation.prev_object_key {
                //
                batch_ids.push(ImageComplex::gen_delete_id());

                batch_payloads.push(TaskPayload::Image(image::ImagePayload::Delete {
                    object_key: prev_object_key.clone(),
                }));

                batch_delays.push(None);
            }

            batch_ids.push(ImageComplex::gen_check_id());

            batch_payloads.push(TaskPayload::Image(image::ImagePayload::CheckUpload {
                resource_kind: image::ResourceKind::ComicCover,
                resource_id: id.clone(),
                object_key: cover_reservation.object_key.clone(),
                version: cover_reservation.cover_version,
                image_hash: transaction_image_hash.clone(),
                image_ext,
            }));

            batch_delays.push(Some(Duration::from_secs(15 * 60)));

            let batch_tasks: Vec<Task<'_, String, TaskPayload>> = batch_ids
                .iter()
                .zip(batch_payloads.iter())
                .zip(batch_delays.iter())
                .map(|((id, payload), delay)| Task {
                    id,
                    payload,
                    delay: *delay,
                })
                .collect();

            prom.step(context, &DeferBatch::new(&batch_tasks)).await?;

            accept((
                cover_reservation.object_key,
                cover_reservation.cover_version,
                true,
            ))
        })
        .await?;

    let slot = match upload_required {
        //
        true => {
            //
            let upload_spec = ImageUploadSpec {
                object_key: &object_key,
                content_type: image_ext.content_type(),
                checksum_sha256: &image_hash,
                content_length: byte_length,
            };

            let upload_slot = image_pool.get_upload_slot(upload_spec).await?;

            Some(ImageUploadSlotVal {
                put_url: upload_slot.url.to_string(),
                image_version: cover_version,
                headers: upload_slot.headers,
            })
        }

        false => None,
    };

    accept(ReserveComicCoverPayload { slot })
}

/// Marks a reserved comic cover as successfully uploaded.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn mark_cover_uploaded<N, C, R, I>(
    nucl: &N,
    repo: &R,
    image_manager: &I,
    token: UserToken,
    id: String,
    params: MarkComicCoverUploadedParams,
) -> BaseResult<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Send + Sync,
    I: ImageManager,
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

    let comic_info = repo
        .run(&GetComicInfo {
            id: &id,
            incls: &[],
        })
        .await?;

    if comic_info.cover_version != params.image_version {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-stale-cover-upload"),
        });
    }

    if comic_info.cover_uploaded {
        return accept(());
    }

    let cover_key =
        comic_info
            .cover_key
            .clone()
            .ok_or_else(|| BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-stale-cover-upload"),
            })?;

    let object_info =
        image_manager
            .head_object(&cover_key)
            .await?
            .ok_or_else(|| BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-stale-cover-upload"),
            })?;

    if object_info.checksum_sha256 != comic_info.cover_hash {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-invalid-image-hash"),
        });
    }

    nucl.coord(async move |context| {
        //
        let locked_comic_info = repo
            .step(
                context,
                &GetComicInfoExcluded {
                    id: &id,
                    incls: &[],
                },
            )
            .await?;

        if locked_comic_info.cover_version != params.image_version
            || locked_comic_info.cover_key.as_deref() != Some(&cover_key)
            || locked_comic_info.cover_hash != comic_info.cover_hash
            || locked_comic_info.cover_ext != comic_info.cover_ext
        {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-stale-cover-upload"),
            });
        }

        repo.step(
            context,
            &MarkComicCoverUploaded {
                id: &id,
                cover_version: params.image_version,
                cover_key: Some(&cover_key),
                cover_uploaded: true,
            },
        )
        .await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Deletes a comic and updates the parent workset counter.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn delete<N, C, R, P>(
    nucl: &N,
    repo: &R,
    prom: &P,
    token: UserToken,
    id: String,
) -> BaseResult<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
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

    nucl.coord(async move |context| {
        //
        ComicComplex::delete_cascade(
            &mut step_proxy! {
                context;
                repo =>
                    for<'a, 'b> GetComicInfoExcluded<'a, 'b>,
                    for<'a> ListChapterInfosExcluded<'a>,
                    for<'a> DeleteComic<'a>,
                    for<'a> UpdateWorksetComicCount<'a>,
                    for<'a, 'b> GetChapterInfoExcluded<'a, 'b>,
                    for<'a> ListPageInfos<'a>,
                    for<'a> DeleteAssignmentInvitations<'a>,
                    for<'a> DeleteAssignments<'a>,
                    for<'a> DeletePages<'a>,
                    for<'a> DeleteChapter<'a>,
                    for<'a> UpdateChapter<'a>,
                    for<'a> UnpinOtherChapters<'a>,
                    for<'a> UpdateComicChapterCount<'a>,
                    for<'a> TouchComicLastActive<'a>,
                    for<'a> ListTermbaseInfosExcluded<'a>,
                    for<'a> GetTermbaseInfoExcluded<'a>,
                    for<'a> DeleteTerms<'a>,
                    for<'a> DeleteTermbase<'a>;
                prom =>
                    for<'a> Defer<'a, String, TaskPayload, ()>,
                    for<'t, 'a> DeferBatch<'t, 'a, String, TaskPayload, ()>;
            },
            &id,
        )
        .await?;

        accept(())
    })
    .await?;

    accept(())
}
