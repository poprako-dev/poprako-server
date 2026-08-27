//! Transactional chapter page-manifest reservation.

/// Page-manifest reservation response data.
pub mod reservation;

use std::time::Duration;

use poprako_orchestra::{Context, OperStep as _};

use crate::complex::chapter::ChapterComplex;
use crate::complex::image::ImageComplex;
use crate::complex::page::PageComplex;
use crate::complex::page::manifest::{ManifestPlan, PageManifestComplex};
use crate::config::ImageConfig;
use crate::model::read::proj::page::PageInfo;
use crate::model::write::page::{PageEntry, PageImageSpec, PageManifestRepl};
use crate::part::prom::Prom;
use crate::part::prom::oper::{Defer, DeferBatch};
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::prom::task::Task;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::chapter::{
    GetChapterInfoExcluded, SetChapterPageCounters,
};
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page::{
    CreatePages, DeletePages, ListPageInfosExcluded, ShiftPageIndexesTemporary,
    UpdatePageManifest,
};
use crate::part::repo::page::PageRepo;
use crate::result::{BaseError, BaseRest, accept};
use crate::usecase::page::reserve::manifest::reservation::{
    PageReservation, PageUploadReservation,
};
use crate::usecase::page::reserve::validation::{
    checked_page_index, invalid_manifest_index, next_image_version,
};
use crate::util::next_snowflake_id;
use crate::value::image::ImageKind;

/// Accumulates database changes and deferred work for one manifest.
pub struct ManifestChanges {
    //
    /// Entries for newly created pages.
    page_entries: Vec<PageEntry>,
    /// Reservations returned for all requested pages.
    reservations: Vec<PageReservation>,
    /// Storage keys scheduled for deletion.
    delete_object_keys: Vec<String>,
    /// Aggregate unit count across retained pages.
    total_unit_count: usize,
    /// Aggregate translated-unit count across retained pages.
    translated_unit_count: usize,
    /// Aggregate proofread-unit count across retained pages.
    proofread_unit_count: usize,
}

// Provides constructors and counter aggregation for manifest changes.
impl ManifestChanges {
    // Creates an empty accumulator sized for the requested manifest.
    fn new(page_count: usize) -> Self {
        //
        Self {
            page_entries: Vec::with_capacity(page_count),
            reservations: Vec::with_capacity(page_count),
            delete_object_keys: Vec::new(),
            total_unit_count: 0,
            translated_unit_count: 0,
            proofread_unit_count: 0,
        }
    }

    // Adds retained-page counters to the manifest totals.
    const fn include_page_counters(&mut self, page_info: &PageInfo) {
        //
        self.total_unit_count += page_info.total_unit_count;

        self.translated_unit_count += page_info.translated_unit_count;

        self.proofread_unit_count += page_info.proofread_unit_count;
    }
}

/// Updates one existing page and prepares its upload reservation.
pub async fn reserve_existing_page<C, R>(
    repo: &R,
    context: &mut C,
    image_config: &ImageConfig,
    chapter_id: &str,
    page_info: &PageInfo,
    page_spec: &PageImageSpec,
    index: usize,
) -> BaseRest<(PageReservation, Option<String>)>
where
    C: Context + Send,
    R: PageRepo<C> + Send + Sync,
{
    //
    let identity_changed = page_info.image_hash.as_ref()
        != Some(&page_spec.image_hash)
        || page_info.image_ext != Some(page_spec.ext);

    if identity_changed && page_spec.new_byte_len.is_none() {
        //
        return Err(ImageComplex::invalid_byte_length_rejection(
            image_config,
            0,
            ImageKind::PageImage,
        ));
    }

    let image_version = next_image_version(page_info, identity_changed)?;

    let image_key = identity_changed
        .then(|| {
            //
            PageComplex::gen_image_key(
                chapter_id,
                &page_info.id,
                image_version,
                page_spec.ext.suffix(),
            )
        })
        .or_else(|| page_info.image_key.clone());

    if !identity_changed && page_info.is_image_uploaded.is_none() {
        //
        return Err(BaseError::Unrecoverable {
            message: "[reserve_chapter_pages] retained page image upload state is missing".into(),
        });
    }

    let image_uploaded =
        !identity_changed && page_info.is_image_uploaded.unwrap_or(false);

    let deleted_object_key = identity_changed
        .then(|| page_info.image_key.clone())
        .flatten();

    let page_manifest_update = PageManifestRepl {
        id: page_info.id.clone(),
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

    let upload = match (image_uploaded, page_spec.new_byte_len) {
        //
        (true, _) | (false, None) => None,

        (false, Some(new_byte_len)) => Some(PageUploadReservation::new(
            image_key.ok_or_else(|| BaseError::Unrecoverable {
                message:
                    "[reserve_chapter_pages] pending page image key is missing"
                        .into(),
            })?,
            new_byte_len,
        )),
    };

    accept((
        PageReservation::new(
            page_info.id.clone(),
            checked_page_index(index)?,
            upload,
            image_version,
            page_spec.image_hash.clone(),
            page_spec.ext,
        ),
        deleted_object_key,
    ))
}

/// Creates one new page and prepares its upload reservation.
pub fn reserve_new_page(
    image_config: &ImageConfig,
    chapter_id: &str,
    page_spec: &PageImageSpec,
    index: usize,
) -> BaseRest<(PageEntry, PageReservation)> {
    //
    let new_byte_len = page_spec.new_byte_len.ok_or_else(|| {
        //
        ImageComplex::invalid_byte_length_rejection(
            image_config,
            0,
            ImageKind::PageImage,
        )
    })?;

    let (page_id, image_version) = (PageComplex::gen_id(), 1);

    let object_key = PageComplex::gen_image_key(
        chapter_id,
        &page_id,
        image_version,
        page_spec.ext.suffix(),
    );

    let page_entry = PageEntry {
        id: page_id.clone(),
        chapter_id: chapter_id.to_owned(),
        index,
        image_key: Some(object_key.clone()),
        image_version,
        image_hash: page_spec.image_hash.clone(),
        image_ext: page_spec.ext,
    };

    let reservation = PageReservation::new(
        page_id,
        checked_page_index(index)?,
        Some(PageUploadReservation::new(object_key, new_byte_len)),
        image_version,
        page_spec.image_hash.clone(),
        page_spec.ext,
    );

    accept((page_entry, reservation))
}

/// Applies requested manifest entries to existing and new pages.
pub async fn apply_manifest_matches<C, R>(
    (repo, context, image_config): (&R, &mut C, &ImageConfig),
    chapter_id: &str,
    user_id: &str,
    page_specs: &[PageImageSpec],
    existing_page_infos: &[PageInfo],
    existing_indexes: &[Option<usize>],
    changes: &mut ManifestChanges,
) -> BaseRest<()>
where
    C: Context + Send,
    R: PageRepo<C> + Send + Sync,
{
    for (index, (page_spec, existing_index)) in
        page_specs.iter().zip(existing_indexes).enumerate()
    {
        //
        let Some(existing_index) = existing_index else {
            //
            let (page_entry, reservation) =
                reserve_new_page(image_config, chapter_id, page_spec, index)?;

            changes.page_entries.push(page_entry);

            changes.reservations.push(reservation);

            continue;
        };

        let page_info =
            existing_page_infos.get(*existing_index).ok_or_else(|| {
                //
                invalid_manifest_index(
                    chapter_id,
                    user_id,
                    *existing_index,
                    existing_page_infos.len(),
                )
            })?;

        changes.include_page_counters(page_info);

        let (reservation, deleted_object_key) = reserve_existing_page(
            repo,
            context,
            image_config,
            chapter_id,
            page_info,
            page_spec,
            index,
        )
        .await?;

        if let Some(object_key) = deleted_object_key {
            changes.delete_object_keys.push(object_key);
        }

        changes.reservations.push(reservation);
    }

    accept(())
}

/// Deletes pages removed from the requested manifest.
pub async fn delete_removed_pages<C, R>(
    repo: &R,
    context: &mut C,
    chapter_id: &str,
    user_id: &str,
    existing_page_infos: &[PageInfo],
    deleted_existing_indexes: &[usize],
    delete_object_keys: &mut Vec<String>,
) -> BaseRest<()>
where
    C: Context + Send,
    R: PageRepo<C> + Send + Sync,
{
    let mut deleted_page_ids =
        Vec::with_capacity(deleted_existing_indexes.len());

    for existing_index in deleted_existing_indexes {
        //
        let page_info =
            existing_page_infos.get(*existing_index).ok_or_else(|| {
                //
                invalid_manifest_index(
                    chapter_id,
                    user_id,
                    *existing_index,
                    existing_page_infos.len(),
                )
            })?;

        if let Some(object_key) = &page_info.image_key {
            delete_object_keys.push(object_key.clone());
        }

        deleted_page_ids.push(page_info.id.clone());
    }

    DeletePages::Ids {
        ids: &deleted_page_ids,
    }
    .step_on(repo, context)
    .await?;

    accept(())
}

/// Defers cleanup and upload verification tasks for manifest changes.
pub async fn defer_image_tasks<C, P>(
    prom: &P,
    context: &mut C,
    delete_object_keys: &[String],
    reservations: &[PageReservation],
) -> BaseRest<()>
where
    C: Context + Send,
    P: Prom<C> + Send + Sync,
{
    let (mut task_ids, mut task_payloads, mut task_delays) =
        (Vec::new(), Vec::new(), Vec::new());

    for object_key in delete_object_keys {
        //
        task_ids.push(ImageComplex::gen_delete_id());

        task_payloads.push(TaskPayload::Image {
            payload: image::ImagePayload::Delete {
                object_key: object_key.clone(),
            },
        });

        task_delays.push(None);
    }

    for reservation in reservations {
        //
        let Some((page_id, object_key, image_version)) =
            reservation.upload_check()
        else {
            continue;
        };

        task_ids.push(ImageComplex::gen_check_id());

        task_payloads.push(TaskPayload::Image {
            payload: image::ImagePayload::CheckUpload {
                image_kind: ImageKind::PageImage,
                resource_id: page_id.to_owned(),
                object_key: object_key.to_owned(),
                version: image_version,
            },
        });

        task_delays.push(Some(Duration::from_mins(15)));
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

    accept(())
}

/// Persists chapter counters and downstream manifest effects.
pub async fn finalize_manifest<C, R, P>(
    repo: &R,
    prom: &P,
    context: &mut C,
    chapter_id: &str,
    comic_id: &str,
    user_id: &str,
    changes: &ManifestChanges,
) -> BaseRest<()>
where
    C: Context + Send,
    R: ChapterRepo<C> + ComicRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
{
    SetChapterPageCounters {
        id: chapter_id,
        page_count: changes.reservations.len(),
        total_unit_count: changes.total_unit_count,
        translated_unit_count: changes.translated_unit_count,
        proofread_unit_count: changes.proofread_unit_count,
    }
    .step_on(repo, context)
    .await?;

    let (advance_id, advance_payload) = (
        next_snowflake_id(),
        TaskPayload::Chapter {
            payload: ChapterPayload::TryAdvanceRawProvideStage {
                chapter_id: chapter_id.to_owned(),
                actor_user_id: Some(user_id.to_owned()),
            },
        },
    );

    let advance_task = Task {
        id: &advance_id,
        payload: &advance_payload,
        delay: Some(Duration::from_mins(20)),
    };

    Defer::new(advance_task).step_on(prom, context).await?;

    TouchComicLastActive { id: comic_id }
        .step_on(repo, context)
        .await?;

    accept(())
}

/// Verifies that manifest planning preserved the request length.
pub fn ensure_manifest_length(
    chapter_id: &str,
    user_id: &str,
    page_spec_count: usize,
    manifest_plan: &ManifestPlan,
) -> BaseRest<()> {
    //
    if manifest_plan.matches.len() == page_spec_count {
        return accept(());
    }

    let err_message =
        String::from("page manifest match count differs from request count");

    tracing::error!(
        err_message = %err_message,
        chapter_id,
        user_id,
        manifest_match_count = manifest_plan.matches.len(),
        page_spec_count,
        "internal invariant violated: invalid page manifest length",
    );

    Err(BaseError::Unrecoverable {
        message: err_message,
    })
}

/// Applies the page-manifest reservation within the caller's transaction.
pub async fn apply_manifest<C, R, P>(
    (repo, prom, image_config, context): (&R, &P, &ImageConfig, &mut C),
    user_id: &str,
    chapter_id: &str,
    page_specs: &[PageImageSpec],
    page_count: usize,
) -> BaseRest<Vec<PageReservation>>
where
    C: Context + Send,
    R: ChapterRepo<C> + ComicRepo<C> + PageRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
{
    //
    // Chapter -> Page is the shared lock order that prevents both deadlocks
    // and page-aggregate counter races.
    let chapter_info = GetChapterInfoExcluded {
        id: chapter_id,
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

    let manifest_plan = PageManifestComplex::build(
        &chapter_info.id,
        &existing_page_infos,
        page_specs,
    )?;

    ensure_manifest_length(
        &chapter_info.id,
        user_id,
        page_specs.len(),
        &manifest_plan,
    )?;

    ShiftPageIndexesTemporary {
        chapter_id: &chapter_info.id,
    }
    .step_on(repo, context)
    .await?;

    let mut changes = ManifestChanges::new(page_count);

    let existing_indexes = manifest_plan
        .matches
        .iter()
        .map(|manifest_match| manifest_match.existing_index)
        .collect::<Vec<_>>();

    apply_manifest_matches(
        (repo, context, image_config),
        &chapter_info.id,
        user_id,
        page_specs,
        &existing_page_infos,
        &existing_indexes,
        &mut changes,
    )
    .await?;

    CreatePages {
        entries: &changes.page_entries,
    }
    .step_on(repo, context)
    .await?;

    delete_removed_pages(
        repo,
        context,
        &chapter_info.id,
        user_id,
        &existing_page_infos,
        &manifest_plan.deleted_existing_indexes,
        &mut changes.delete_object_keys,
    )
    .await?;

    defer_image_tasks(
        prom,
        context,
        &changes.delete_object_keys,
        &changes.reservations,
    )
    .await?;

    finalize_manifest(
        repo,
        prom,
        context,
        &chapter_info.id,
        &chapter_info.comic_id,
        user_id,
        &changes,
    )
    .await?;

    accept(changes.reservations)
}
