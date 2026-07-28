//! Complete chapter page-manifest reservation.

use std::collections::HashSet;
use std::time::Duration;

use poprako_orchestra::{Nucl, OperStep as _, run_proxy};
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use poprako_orchestra_extra::prom::task::Task;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::image::ImageComplex;
use crate::complex::page::manifest::build;
use crate::complex::page::{PageComplex, PagePermComplex};
use crate::data::instr::page::ReserveChapterPagesInstr;
use crate::data::val::page::{ReserveChapterPagesVal, ReservedPageVal};
use crate::data::view::image::ImageUploadSlotView;
use crate::model::shared::user::UserToken;
use crate::model::write::page::{PageEntry, PageImageSpec, PageManifestRepl};
use crate::part::image::{ImagePool, ImageUploadSpec};
use crate::part::prom::Prom;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    GetChapterInfoExcluded, SetChapterPageCounters,
};
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page::{
    CreatePages, DeletePages, ListPageInfosExcluded, ShiftPageIndexesTemporary,
    UpdatePageManifest,
};
use crate::part::repo::page::PageRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::image::{ImageExt, ImageHash};

/// Validates that the page count is in the valid range.
///
/// The maximum is 200 because page reservation for a single chapter can never
/// exceed this number — the manifest-based flow sets a hard cap for practical
/// upload and review capacity.
pub fn validate_page_count(page_count: i32) -> BaseRest<()> {
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
#[instrument(level = "info", err(Debug), skip(nucl, repo, prom, image_pool))]
pub async fn reserve_chapter_pages<N, C, R, P, I>(
    (nucl, repo, prom, image_pool): (&N, &R, &P, &I),
    token: UserToken,
    instr: ReserveChapterPagesInstr,
) -> BaseRest<ReserveChapterPagesVal>
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
    let ReserveChapterPagesInstr { chapter_id, pages } = instr;

    let page_specs = pages
        .into_iter()
        .map(PageImageSpec::from)
        .collect::<Vec<_>>();

    let page_count =
        i32::try_from(page_specs.len()).map_err(|_| BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-invalid-page-count"),
        })?;

    validate_page_count(page_count)?;

    for new_byte_len in page_specs
        .iter()
        .filter_map(|page_spec| page_spec.new_byte_len)
    {
        ImageComplex::ensure_byte_length(
            new_byte_len,
            image::ResourceKind::PageImage,
        )?;
    }

    let mut explicit_page_ids = HashSet::new();

    for page_spec in &page_specs {
        //
        let Some(page_id) = &page_spec.page_id else {
            continue;
        };

        if !explicit_page_ids.insert(page_id) {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-duplicate-page-id"),
            });
        }
    }

    // Holds one requested upload target within a page reservation.
    struct PageUploadReservation {
        //
        // Upload destination in object storage.
        object_key: String,
        // Expected size (in bytes) for capacity pre-allocation and verification.
        new_byte_len: u64,
    }

    // Holds the identity and optional upload request for one reserved page.
    struct PageReservation {
        //
        // Page identifier for this reservation.
        page_id: String,
        // Ordering index used to keep reservation payload deterministic.
        index: u32,

        // Optional upload metadata when the caller requests a new page image.
        upload: Option<PageUploadReservation>,
        // Monotonic image revision value after reservation.
        image_version: u32,
        // Expected image digest for reservation integrity checks.
        image_hash: ImageHash,
        // File extension used for generated object key and downstream image handling.
        ext: ImageExt,
    }

    PagePermComplex::ensure_user_can_reserve(
        &mut run_proxy! {
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &chapter_id,
    )
    .await?;

    let reservations = nucl
        .coord(async move |context| {
            //
            // NOTE: Chapter -> Page is the shared lock order that prevents
            // both deadlocks and page-aggregate counter races.
            let chapter_info = GetChapterInfoExcluded {
                id: &chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            let existing_page_infos = ListPageInfosExcluded {
                chapter_id: &chapter_info.id,
            }
            .step_on(repo, context)
            .await?;

            let manifest_plan = build(
                &chapter_info.id,
                &existing_page_infos,
                &page_specs,
            )?;

            ShiftPageIndexesTemporary {
                chapter_id: &chapter_info.id,
            }
            .step_on(repo, context)
            .await?;

            let mut page_entries =
                Vec::with_capacity(page_count as usize);

            let mut reservations =
                Vec::with_capacity(page_count as usize);

            let mut delete_object_keys = Vec::new();

            let mut total_unit_count = 0;

            let mut translated_unit_count = 0;

            let mut proofread_unit_count = 0;

            for (raw_index, page_spec) in page_specs.iter().enumerate() {
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
                        existing_page_info.image_hash != page_spec.image_hash
                            || existing_page_info.image_ext != page_spec.ext;

                    if identity_changed && page_spec.new_byte_len.is_none() {
                        return Err(BaseError::Expected {
                            variant: ExpectedVariant::Args,
                            message: trl("error-invalid-image-byte-length"),
                        });
                    }

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
                            page_spec.ext.suffix(),
                        )),

                        false => existing_page_info.image_key.clone(),
                    };

                    let image_uploaded = match identity_changed {
                        //
                        true => false,

                        false => existing_page_info.is_image_uploaded,
                    };

                    if identity_changed
                        && let Some(object_key) = &existing_page_info.image_key
                    {
                        delete_object_keys.push(object_key.clone());
                    }

                    let page_manifest_update = PageManifestRepl {
                        id: existing_page_info.id.clone(),
                        index,
                        image_key: image_key.clone(),
                        is_image_uploaded: image_uploaded,
                        image_version,
                        image_hash: page_spec.image_hash.clone(),
                        image_ext: page_spec.ext,
                    };

                    UpdatePageManifest {
                        update: &page_manifest_update,
                    }
                    .step_on(repo, context)
                    .await?;

                    reservations.push(PageReservation {
                        page_id: existing_page_info.id.clone(),
                        index: u32::try_from(index).map_err(|_| BaseError::Unrecoverable {
                            message: "[reserve_chapter_pages] page index must be non-negative".into(),
                        })?,
                        upload: match (image_uploaded, page_spec.new_byte_len) {
                            //
                            (true, _) | (false, None) => None,

                            (false, Some(new_byte_len)) => {
                                Some(PageUploadReservation {
                                    object_key: image_key.ok_or_else(|| {
                                        BaseError::Unrecoverable {
                                            message: "[reserve_chapter_pages] pending page image key is missing"
                                                .into(),
                                        }
                                    })?,
                                    new_byte_len,
                                })
                            }
                        },
                        image_version,
                        image_hash: page_spec.image_hash.clone(),
                        ext: page_spec.ext,
                    });

                    continue;
                }

                let new_byte_len = page_spec.new_byte_len.ok_or_else(|| {
                    BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: trl("error-invalid-image-byte-length"),
                    }
                })?;

                let page_id = PageComplex::gen_id();

                let image_version = 1;

                let object_key = PageComplex::gen_image_key(
                    &chapter_info.id,
                    &page_id,
                    image_version,
                    page_spec.ext.suffix(),
                );

                let page_entry = PageEntry {
                    id: page_id.clone(),
                    chapter_id: chapter_info.id.clone(),
                    index,
                    image_key: Some(object_key.clone()),
                    image_version,
                    image_hash: page_spec.image_hash.clone(),
                    image_ext: page_spec.ext,
                };

                page_entries.push(page_entry);

                reservations.push(PageReservation {
                    page_id,
                    index: u32::try_from(index).map_err(|_| BaseError::Unrecoverable {
                        message: "[reserve_chapter_pages] page index must be non-negative".into(),
                    })?,
                    upload: Some(PageUploadReservation {
                        object_key,
                        new_byte_len,
                    }),
                    image_version,
                    image_hash: page_spec.image_hash.clone(),
                    ext: page_spec.ext,
                });
            }

            CreatePages {
                entries: &page_entries,
            }
            .step_on(repo, context)
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

            DeletePages::Ids {
                ids: &deleted_page_ids,
            }
            .step_on(repo, context)
            .await?;

            let mut task_ids = Vec::new();

            let mut task_payloads = Vec::new();

            let mut task_delays = Vec::new();

            for object_key in &delete_object_keys {
                //
                task_ids.push(ImageComplex::gen_delete_id());

                task_payloads.push(TaskPayload::Image(image::ImagePayload::Delete {
                    object_key: object_key.clone(),
                }));

                task_delays.push(None);
            }

            for reservation in &reservations {
                //
                let Some(upload) = &reservation.upload else {
                    continue;
                };

                task_ids.push(ImageComplex::gen_check_id());

                task_payloads.push(TaskPayload::Image(
                    image::ImagePayload::CheckUpload {
                        resource_kind: image::ResourceKind::PageImage,
                        resource_id: reservation.page_id.clone(),
                        object_key: upload.object_key.clone(),
                        version: reservation.image_version,
                    },
                ));

                task_delays.push(Some(Duration::from_secs(15 * 60)));
            }

            let image_tasks = task_ids
                .iter()
                .zip(task_payloads.iter())
                .zip(task_delays.iter())
                .map(|((id, payload), delay)| Task {
                    id,
                    payload,
                    delay: *delay,
                })
                .collect::<Vec<Task<'_, String, TaskPayload>>>();

            DeferBatch::new(&image_tasks).step_on(prom, context).await?;

            SetChapterPageCounters {
                id: &chapter_info.id,
                page_count: i32::try_from(reservations.len()).map_err(|_| BaseError::Unrecoverable {
                    message: "[reserve_chapter_pages] page count exceeds i32".into(),
                })?,
                total_unit_count,
                translated_unit_count,
                proofread_unit_count,
            }
            .step_on(repo, context)
            .await?;

            let advance_id = next_snowflake_id();

            let advance_payload =
                TaskPayload::Chapter(ChapterPayload::TryAdvanceRawProvideStage {
                    chapter_id: chapter_info.id.clone(),
                });

            let advance_task = Task {
                id: &advance_id,
                payload: &advance_payload,
                delay: Some(Duration::from_secs(20 * 60)),
            };

            Defer::new(advance_task).step_on(prom, context).await?;

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            accept(reservations)
        })
        .await?;

    let pages = futures_util::future::join_all(reservations.into_iter().map(
        |reservation| async move {
            //
            let slot = match reservation.upload {
                //
                Some(upload) => {
                    //
                    let upload_spec = ImageUploadSpec {
                        object_key: &upload.object_key,
                        content_type: reservation.ext.content_type(),
                        content_length: upload.new_byte_len,
                    };

                    let upload_target =
                        image_pool.get_upload_slot(upload_spec).await?;

                    Some(ImageUploadSlotView {
                        put_url: upload_target.url.to_string(),
                        image_version: reservation.image_version,
                        headers: upload_target.headers,
                    })
                }

                None => None,
            };

            accept(ReservedPageVal {
                page_id: reservation.page_id,
                index: reservation.index,
                image_hash: reservation.image_hash,
                ext: reservation.ext,
                slot,
            })
        },
    ))
    .await
    .into_iter()
    .collect::<BaseRest<Vec<_>>>()?;

    accept(ReserveChapterPagesVal { pages })
}
