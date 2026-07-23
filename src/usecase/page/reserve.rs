//! Complete chapter page-manifest reservation.

use std::collections::HashSet;
use std::time::Duration;

use poprako_orchestra::{Nucl, run_proxy};
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use poprako_orchestra_extra::prom::task::Task;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::image::ImageComplex;
use crate::complex::page::manifest::build;
use crate::complex::page::{PageComplex, PagePermComplex};
use crate::data::page::{
    ReserveChapterPagesParams, ReserveChapterPagesPayload, ReservedPagePayload,
    PageSlotVal,
};
use crate::model::page::{PageEntry, PageManifestUpdate};
use crate::model::user::UserToken;
use crate::part::image::{ImagePool, ImageUploadSpec};
use crate::part::prom::Prom;
use crate::part::prom::payload::chapter::CheckUploadFinish;
use crate::part::prom::payload::{Payload, image};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    CompleteChapterRawProvide, GetChapterInfoExcluded, ResetChapterRawProvide,
    SetChapterPageCounters,
};
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page::{
    CreatePages, DeletePages, ListPageInfosExcluded, ShiftPageIndexesTemporary,
    UpdatePageManifest,
};
use crate::part::repo::page::PageRepo;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::image::{ImageExt, ImageHash};

const MAX_IMAGE_BYTE_LENGTH: u64 = 20 * 1024 * 1024;

/// Validates that the image byte length is within range.
pub(super) fn validate_image_byte_length(byte_length: u64) -> BaseResult<()> {
    //
    if !(1..=MAX_IMAGE_BYTE_LENGTH).contains(&byte_length) {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-invalid-image-byte-length"),
        });
    }

    accept(())
}

/// Validates that the page count is in the valid range.
///
/// The maximum is 200 because page reservation for a single chapter can never
/// exceed this number — the manifest-based flow sets a hard cap for practical
/// upload and review capacity.
pub(super) fn validate_page_count(page_count: i32) -> BaseResult<()> {
    //
    if !(1..=200).contains(&page_count) {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-invalid-page-count"),
        });
    }

    accept(())
}

/// Reserves upload slots for all pages in an empty chapter.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn reserve_chapter_pages<N, C, R, P, I>(
    nucl: &N,
    repo: &R,
    prom: &P,
    image_pool: &I,
    token: UserToken,
    params: ReserveChapterPagesParams,
) -> BaseResult<ReserveChapterPagesPayload>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: ChapterRepo<C>
        + ComicRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    let page_count =
        i32::try_from(params.pages.len()).map_err(|_| BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-invalid-page-count"),
        })?;

    validate_page_count(page_count)?;

    for page_input in &params.pages {
        validate_image_byte_length(page_input.byte_length)?;
    }

    let mut explicit_page_ids = HashSet::new();

    for page_input in &params.pages {
        //
        let Some(page_id) = &page_input.page_id else {
            continue;
        };

        if !explicit_page_ids.insert(page_id) {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-duplicate-page-id"),
            });
        }
    }

    /// Holds the ID, storage key, and version for one reserved page upload.
    struct PageReservation {
        page_id: String,
        index: u32,
        object_key: Option<String>,
        image_version: u32,
        image_hash: ImageHash,
        byte_length: u64,
        ext: ImageExt,
    }

    PagePermComplex::ensure_user_can_reserve(
        &mut run_proxy! {
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &params.chapter_id,
    )
    .await?;

    let reservations = nucl
        .coord(async move |context| {
            //
            let chapter_info = repo
                .step(
                    context,
                    &GetChapterInfoExcluded {
                        id: &params.chapter_id,
                        incls: &[],
                    },
                )
                .await?;

            ChapterComplex::ensure_user_write_allowed(&chapter_info)?;

            let existing_page_infos = repo
                .step(
                    context,
                    &ListPageInfosExcluded {
                        chapter_id: &chapter_info.id,
                    },
                )
                .await?;

            let manifest_plan = build(
                &chapter_info.id,
                &existing_page_infos,
                &params.pages,
            )?;

            repo.step(
                context,
                &ShiftPageIndexesTemporary {
                    chapter_id: &chapter_info.id,
                },
            )
            .await?;

            let mut page_entries =
                Vec::with_capacity(page_count as usize);

            let mut reservations =
                Vec::with_capacity(page_count as usize);

            let mut delete_object_keys = Vec::new();

            let mut total_unit_count = 0;

            let mut translated_unit_count = 0;

            let mut proofread_unit_count = 0;

            for (raw_index, page_input) in params.pages.iter().enumerate() {
                //
                let index = i32::try_from(raw_index).map_err(|_| BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: trl("error-invalid-page-count"),
                })?;

                let existing_page_info = manifest_plan.matches[raw_index]
                    .existing_index
                    .map(|existing_index| &existing_page_infos[existing_index]);

                if let Some(existing_page_info) = existing_page_info {
                    //
                    total_unit_count += existing_page_info.total_unit_count;

                    translated_unit_count +=
                        existing_page_info.translated_unit_count;

                    proofread_unit_count += existing_page_info.proofread_unit_count;

                    let identity_changed =
                        existing_page_info.image_hash != page_input.image_hash;

                    let image_version = match identity_changed {
                        //
                        true => existing_page_info
                            .image_version
                            .checked_add(1)
                            .ok_or_else(|| BaseError::Unrecoverable {
                                message: "[reserve_chapter_pages] image version overflow"
                                    .into(),
                            })?,

                        false => existing_page_info.image_version,
                    };

                    let image_key = match identity_changed {
                        //
                        true => Some(PageComplex::gen_image_key(
                            &chapter_info.id,
                            &existing_page_info.id,
                            image_version,
                            page_input.ext.suffix(),
                        )),

                        false => existing_page_info.image_key.clone(),
                    };

                    let image_uploaded = match identity_changed {
                        //
                        true => false,

                        false => existing_page_info.image_uploaded,
                    };

                    if identity_changed
                        && let Some(object_key) = &existing_page_info.image_key
                    {
                        delete_object_keys.push(object_key.clone());
                    }

                    let page_manifest_update = PageManifestUpdate {
                        id: existing_page_info.id.clone(),
                        index,
                        image_key: image_key.clone(),
                        image_uploaded,
                        image_version,
                        image_hash: page_input.image_hash.clone(),
                        image_byte_len: page_input.byte_length,
                        image_ext: page_input.ext,
                    };

                    repo.step(
                        context,
                        &UpdatePageManifest {
                            update: &page_manifest_update,
                        },
                    )
                    .await?;

                    reservations.push(PageReservation {
                        page_id: existing_page_info.id.clone(),
                        index: u32::try_from(index).map_err(|_| BaseError::Unrecoverable {
                            message: "[reserve_chapter_pages] page index must be non-negative".into(),
                        })?,
                        object_key: match image_uploaded {
                            //
                            true => None,

                            false => Some(image_key.ok_or_else(|| {
                                BaseError::Unrecoverable {
                                    message: "[reserve_chapter_pages] pending page image key is missing"
                                        .into(),
                                }
                            })?),
                        },
                        image_version,
                        image_hash: page_input.image_hash.clone(),
                        byte_length: page_input.byte_length,
                        ext: page_input.ext,
                    });

                    continue;
                }

                let page_id = PageComplex::gen_id();

                let image_version = 1;

                let object_key = PageComplex::gen_image_key(
                    &chapter_info.id,
                    &page_id,
                    image_version,
                    page_input.ext.suffix(),
                );

                let page_entry = PageEntry {
                    id: page_id.clone(),
                    chapter_id: chapter_info.id.clone(),
                    index,
                    image_key: Some(object_key.clone()),
                    image_version,
                    image_hash: page_input.image_hash.clone(),
                    image_byte_len: page_input.byte_length,
                    image_ext: page_input.ext,
                };

                page_entries.push(page_entry);

                reservations.push(PageReservation {
                    page_id,
                    index: u32::try_from(index).map_err(|_| BaseError::Unrecoverable {
                        message: "[reserve_chapter_pages] page index must be non-negative".into(),
                    })?,
                    object_key: Some(object_key),
                    image_version,
                    image_hash: page_input.image_hash.clone(),
                    byte_length: page_input.byte_length,
                    ext: page_input.ext,
                });
            }

            repo.step(
                context,
                &CreatePages {
                    entries: &page_entries,
                },
            )
            .await?;

            let deleted_page_ids = manifest_plan
                .deleted_existing_indexes
                .iter()
                .map(|existing_index| {
                    //
                    let page_info = &existing_page_infos[*existing_index];

                    if let Some(object_key) = &page_info.image_key {
                        delete_object_keys.push(object_key.clone());
                    }

                    page_info.id.clone()
                })
                .collect::<Vec<_>>();

            repo.step(
                context,
                &DeletePages::Ids {
                    ids: &deleted_page_ids,
                },
            )
            .await?;

            let mut task_ids = Vec::new();

            let mut task_payloads = Vec::new();

            let mut task_delays = Vec::new();

            for object_key in &delete_object_keys {
                //
                task_ids.push(ImageComplex::gen_delete_id());

                task_payloads.push(Payload::Image(image::Payload::Delete {
                    object_key: object_key.clone(),
                }));

                task_delays.push(None);
            }

            for reservation in &reservations {
                //
                let Some(object_key) = &reservation.object_key else {
                    continue;
                };

                task_ids.push(ImageComplex::gen_check_id());

                task_payloads.push(Payload::Image(
                    image::Payload::CheckUpload {
                        resource_kind: image::ResourceKind::PageImage,
                        resource_id: reservation.page_id.clone(),
                        object_key: object_key.clone(),
                        version: reservation.image_version,
                    },
                ));

                task_delays.push(Some(Duration::from_secs(15 * 60)));
            }

            let image_tasks: Vec<Task<'_, String, Payload>> = task_ids
                .iter()
                .zip(task_payloads.iter())
                .zip(task_delays.iter())
                .map(|((id, payload), delay)| Task {
                    id,
                    payload,
                    delay: *delay,
                })
                .collect();

            prom.step(context, &DeferBatch::new(&image_tasks)).await?;

            repo.step(
                context,
                &SetChapterPageCounters {
                    id: &chapter_info.id,
                    page_count: i32::try_from(reservations.len()).map_err(|_| BaseError::Unrecoverable {
                        message: "[reserve_chapter_pages] page count exceeds i32".into(),
                    })?,
                    total_unit_count,
                    translated_unit_count,
                    proofread_unit_count,
                },
            )
            .await?;

            let has_pending_page = reservations
                .iter()
                .any(|reservation| reservation.object_key.is_some());

            match has_pending_page {
                //
                true => {
                    //
                    repo.step(
                        context,
                        &ResetChapterRawProvide {
                            id: &chapter_info.id,
                        },
                    )
                    .await?;

                    let advance_id = next_snowflake_id();

                    let advance_payload =
                        Payload::CheckChapterUploadFinish(CheckUploadFinish {
                            chapter_id: chapter_info.id.clone(),
                        });

                    let advance_task = Task {
                        id: &advance_id,
                        payload: &advance_payload,
                        delay: Some(Duration::from_secs(20 * 60)),
                    };

                    prom.step(context, &Defer::new(advance_task)).await?;
                }

                false => {
                    repo.step(
                        context,
                        &CompleteChapterRawProvide {
                            id: &chapter_info.id,
                        },
                    )
                    .await?;
                }
            }

            repo.step(
                context,
                &TouchComicLastActive {
                    id: &chapter_info.comic_id,
                },
            )
            .await?;

            accept(reservations)
        })
        .await?;

    let pages = futures_util::future::join_all(reservations.into_iter().map(
        |reservation| async move {
            //
            let slot = match reservation.object_key {
                //
                Some(object_key) => {
                    //
                    let upload_spec = ImageUploadSpec {
                        object_key: &object_key,
                        content_type: reservation.ext.content_type(),
                        checksum_sha256: &reservation.image_hash,
                        content_length: reservation.byte_length,
                    };

                    let upload_target =
                        image_pool.get_upload_slot(upload_spec).await?;

                    Some(PageSlotVal {
                        put_url: upload_target.url.to_string(),
                        image_version: reservation.image_version,
                        headers: upload_target.headers,
                    })
                }

                None => None,
            };

            accept(ReservedPagePayload {
                page_id: reservation.page_id,
                index: reservation.index,
                image_hash: reservation.image_hash,
                byte_length: reservation.byte_length,
                ext: reservation.ext,
                slot,
            })
        },
    ))
    .await
    .into_iter()
    .collect::<BaseResult<Vec<_>>>()?;

    accept(ReserveChapterPagesPayload { pages })
}
