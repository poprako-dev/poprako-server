//! Handler for the "image" prom topic.
//!
//! Dispatches image [`Payload`] variants to their concrete implementations.

use poprako_orchestra::Nucl;
use tracing::instrument;

use crate::part::image::{ImageManager, ImageObjectInfo};
use crate::part::prom::payload::image::{Payload, ResourceKind};
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::page::{
    GetPageInfo, GetPageInfoExcluded, MarkPageImageUploaded,
    SetPageImageUploaded,
};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::part_impl::prom::rdb_impl::handler::image::resource::ResourceState;
use crate::part_impl::prom::rdb_impl::handler::task_flow::TaskFlow;
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult, accept};
use crate::value::image::{ImageExt, ImageHash};

mod resource;

/// Dispatch an image [`Payload`] to its concrete handler.
#[instrument(level = "info", skip_all)]
pub async fn handle<N, R, I>(
    nucl: &N,
    repo: &R,
    image_pool: &I,
    task: &Payload,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError>,
    R: ChapterRepo<RdbContext>
        + UserRepo<RdbContext>
        + TeamRepo<RdbContext>
        + ComicRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync,
    I: ImageManager + Send + Sync,
{
    match task {
        //
        Payload::CheckUpload {
            resource_kind,
            resource_id,
            object_key,
            version,
            image_hash,
            image_ext,
        } => {
            handle_check_uploaded(
                nucl,
                repo,
                image_pool,
                *resource_kind,
                resource_id,
                object_key,
                *version,
                image_hash,
                *image_ext,
            )
            .await
        }

        Payload::Delete { object_key } => {
            handle_delete(image_pool, object_key).await
        }
    }
}

/// Verifies that an uploaded image object exists and confirms current DB ownership.
#[instrument(level = "info", skip_all)]
async fn handle_check_uploaded<N, R, I>(
    nucl: &N,
    repo: &R,
    image_pool: &I,
    kind: ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
    image_hash: &ImageHash,
    image_ext: ImageExt,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError>,
    R: ChapterRepo<RdbContext>
        + UserRepo<RdbContext>
        + TeamRepo<RdbContext>
        + ComicRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync,
    I: ImageManager + Send + Sync,
{
    let object_info = match image_pool.head_object(object_key).await {
        //
        Ok(object_info) => object_info,

        Err(error) => return TaskFlow::Retry(format!("{:?}", error)),
    };

    match object_info {
        //
        None => match kind {
            //
            ResourceKind::PageImage => {
                process_unverified_page_image(
                    nucl,
                    repo,
                    resource_id,
                    object_key,
                    image_version,
                    image_hash,
                    image_ext,
                )
                .await
            }

            _ => {
                process_missing_image(
                    nucl,
                    repo,
                    kind,
                    resource_id,
                    object_key,
                    image_version,
                    image_hash,
                    image_ext,
                )
                .await
            }
        },

        Some(object_info) => {
            //
            if kind == ResourceKind::PageImage {
                return process_existing_page_image(
                    nucl,
                    repo,
                    image_pool,
                    resource_id,
                    object_key,
                    image_version,
                    object_info,
                    image_hash,
                    image_ext,
                )
                .await;
            }

            process_existing_image(
                nucl,
                repo,
                image_pool,
                kind,
                resource_id,
                object_key,
                image_version,
                object_info,
                image_hash,
                image_ext,
            )
            .await
        }
    }
}

#[instrument(level = "info", skip_all)]
async fn process_missing_image<N, R>(
    nucl: &N,
    repo: &R,
    kind: ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
    image_hash: &ImageHash,
    image_ext: ImageExt,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError>,
    R: UserRepo<RdbContext>
        + TeamRepo<RdbContext>
        + ComicRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync,
{
    match resource::mark_current_or_classify(
        nucl,
        repo,
        kind,
        resource_id,
        object_key,
        image_version,
        image_hash,
        image_ext,
        false,
    )
    .await
    {
        //
        Ok(
            ResourceState::Current
            | ResourceState::Stale
            | ResourceState::Missing,
        ) => TaskFlow::Complete,

        Ok(ResourceState::Mismatched) => TaskFlow::Dead(
            "prom image identity does not match current resource".into(),
        ),

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}

#[instrument(level = "info", skip_all)]
async fn process_existing_page_image<N, R, I>(
    nucl: &N,
    repo: &R,
    image_pool: &I,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
    object_info: ImageObjectInfo,
    image_hash: &ImageHash,
    image_ext: ImageExt,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError>,
    R: ChapterRepo<RdbContext> + PageRepo<RdbContext> + Send + Sync,
    I: ImageManager + Send + Sync,
{
    let page_info = match repo.run(&GetPageInfo { id: resource_id }).await {
        //
        Ok(page_info) => page_info,

        Err(BaseError::Expected { .. }) => {
            // SAFETY: stale payload keys are cleaned only by dedicated Delete tasks.
            return TaskFlow::Complete;
        }

        Err(error) => return TaskFlow::Retry(format!("{:?}", error)),
    };

    match (
        page_info.image_version == image_version,
        page_info.image_key.as_deref() == Some(object_key),
    ) {
        //
        (false, _) => return TaskFlow::Complete,

        (true, false) => {
            return TaskFlow::Dead(
                "prom page image version matches but object key differs".into(),
            );
        }

        (true, true) => {}
    }

    if page_info.image_hash != *image_hash || page_info.image_ext != image_ext {
        return TaskFlow::Dead(
            "prom page image payload identity differs from current resource"
                .into(),
        );
    }

    if *image_hash != object_info.checksum_sha256 {
        //
        let verification_outcome = process_unverified_page_image(
            nucl,
            repo,
            resource_id,
            object_key,
            image_version,
            image_hash,
            image_ext,
        )
        .await;

        match verification_outcome {
            //
            TaskFlow::Complete => {
                return handle_delete(image_pool, object_key).await;
            }

            _ => return verification_outcome,
        }
    }

    let outcome: BaseResult<()> = nucl
        .coord(async move |context| {
            //
            // NOTE: Chapter -> Page is the shared lock order that prevents
            // both deadlocks and chapter upload-summary races.
            repo.step(
                context,
                &GetChapterInfoExcluded {
                    id: &page_info.chapter_id,
                    incls: &[],
                },
            )
            .await?;

            let locked_page_info = repo
                .step(context, &GetPageInfoExcluded { id: resource_id })
                .await?;

            if locked_page_info.image_version != image_version
                || locked_page_info.image_key.as_deref() != Some(object_key)
                || locked_page_info.image_hash != page_info.image_hash
                || locked_page_info.image_ext != page_info.image_ext
            {
                return Err(BaseError::Expected {
                    variant: crate::result::ExpectedVariant::Args,
                    message: "stale page image identity".into(),
                });
            }

            repo.step(
                context,
                &MarkPageImageUploaded {
                    id: resource_id,
                    image_version,
                    image_key: Some(object_key),
                },
            )
            .await?;

            accept(())
        })
        .await
        .map_err(Into::into);

    match outcome {
        //
        Ok(()) => TaskFlow::Complete,

        Err(BaseError::Expected { .. }) => TaskFlow::Complete,

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}

/// Clears an unverified current page image without changing chapter workflow.
#[instrument(level = "info", skip_all)]
async fn process_unverified_page_image<N, R>(
    nucl: &N,
    repo: &R,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
    image_hash: &ImageHash,
    image_ext: ImageExt,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError>,
    R: ChapterRepo<RdbContext> + PageRepo<RdbContext> + Send + Sync,
{
    let page_info = match repo.run(&GetPageInfo { id: resource_id }).await {
        //
        Ok(page_info) => page_info,

        Err(BaseError::Expected { .. }) => return TaskFlow::Complete,

        Err(error) => return TaskFlow::Retry(format!("{:?}", error)),
    };

    match (
        page_info.image_version == image_version,
        page_info.image_key.as_deref() == Some(object_key),
    ) {
        //
        (false, _) => return TaskFlow::Complete,

        (true, false) => {
            return TaskFlow::Dead(
                "prom page image version matches but object key differs".into(),
            );
        }

        (true, true) => {}
    }

    if page_info.image_hash != *image_hash || page_info.image_ext != image_ext {
        return TaskFlow::Dead(
            "prom page image payload identity differs from current resource"
                .into(),
        );
    }

    let outcome: BaseResult<()> = nucl
        .coord(async move |context| {
            //
            // NOTE: Chapter -> Page is the shared lock order that prevents
            // both deadlocks and chapter upload-summary races.
            repo.step(
                context,
                &GetChapterInfoExcluded {
                    id: &page_info.chapter_id,
                    incls: &[],
                },
            )
            .await?;

            let locked_page_info = repo
                .step(context, &GetPageInfoExcluded { id: resource_id })
                .await?;

            if locked_page_info.image_version != image_version
                || locked_page_info.image_key.as_deref() != Some(object_key)
                || locked_page_info.image_hash != page_info.image_hash
                || locked_page_info.image_ext != page_info.image_ext
            {
                return Err(BaseError::Expected {
                    variant: crate::result::ExpectedVariant::Args,
                    message: "stale page image identity".into(),
                });
            }

            repo.step(
                context,
                &SetPageImageUploaded {
                    id: resource_id,
                    image_version,
                    image_key: object_key,
                    image_uploaded: false,
                },
            )
            .await?;

            accept(())
        })
        .await
        .map_err(Into::into);

    match outcome {
        //
        Ok(()) | Err(BaseError::Expected { .. }) => TaskFlow::Complete,

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}

#[instrument(level = "info", skip_all)]
async fn process_existing_image<N, R, I>(
    nucl: &N,
    repo: &R,
    image_pool: &I,
    kind: ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
    object_info: ImageObjectInfo,
    image_hash: &ImageHash,
    image_ext: ImageExt,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError>,
    R: ChapterRepo<RdbContext>
        + UserRepo<RdbContext>
        + TeamRepo<RdbContext>
        + ComicRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync,
    I: ImageManager + Send + Sync,
{
    let resource_state = resource::mark_current_or_classify(
        nucl,
        repo,
        kind,
        resource_id,
        object_key,
        image_version,
        image_hash,
        image_ext,
        object_info.checksum_sha256 == *image_hash,
    )
    .await;

    match resource_state {
        //
        Ok(ResourceState::Current)
            if object_info.checksum_sha256 != *image_hash =>
        {
            handle_delete(image_pool, object_key).await
        }

        Ok(ResourceState::Current) | Ok(ResourceState::Stale) => {
            TaskFlow::Complete
        }

        // SAFETY: stale and deleted resource keys belong exclusively to
        // dedicated Delete tasks.
        Ok(ResourceState::Missing) => TaskFlow::Complete,

        Ok(ResourceState::Mismatched) => TaskFlow::Dead(
            "prom image identity does not match current resource".into(),
        ),

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}

/// Deletes an image object from the storage backend.
#[instrument(level = "info", skip_all)]
async fn handle_delete<I>(image_pool: &I, object_key: &str) -> TaskFlow
where
    I: ImageManager + Send + Sync,
{
    match image_pool.delete_object(object_key).await {
        //
        Ok(()) => TaskFlow::Complete,

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}
