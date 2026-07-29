//! Handler for the "image" prom topic.
//!
//! Dispatches image [`ImagePayload`] variants to their concrete implementations.

use poprako_orchestra::Nucl;
use tracing::instrument;

use crate::part::image::ImageManager;
use crate::part::prom::payload::image::{ImagePayload, ResourceKind};
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
use crate::part_impl::prom::rdb_impl::handler::image::identity::ImageIdentity;
use crate::part_impl::prom::rdb_impl::handler::image::resource::ResourceState;
use crate::part_impl::prom::rdb_impl::handler::task_flow::TaskFlow;
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};

// Internal organization of the `identity` module.
mod identity;
// Internal organization of the `resource` module.
mod resource;

/// Dispatch an image [`ImagePayload`] to its concrete handler.
#[instrument(level = "info", skip_all)]
pub async fn handle<N, R, I>(
    nucl: &N,
    repo: &R,
    image_pool: &I,
    task: &ImagePayload,
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
        // Internal implementation detail.
        ImagePayload::CheckUpload {
            resource_kind,
            resource_id,
            object_key,
            version,
        } => {
            //
            // Internal implementation detail.
            let image_identity = ImageIdentity {
                kind: *resource_kind,
                resource_id,
                object_key,
                version: *version,
            };

            handle_check_uploaded(nucl, repo, image_pool, image_identity).await
        }

        ImagePayload::Delete { object_key } => {
            handle_delete(image_pool, object_key).await
        }
    }
}

/// Verifies that an uploaded image object exists and confirms current DB ownership.
#[instrument(level = "info", skip_all)]
// Internal implementation of `handle_check_uploaded`.
async fn handle_check_uploaded<N, R, I>(
    nucl: &N,
    repo: &R,
    image_pool: &I,
    image_identity: ImageIdentity<'_>,
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
    let object_exists =
        match image_pool.object_exists(image_identity.object_key).await {
            //
            // Internal implementation detail.
            Ok(object_info) => object_info,

            Err(error) => return TaskFlow::Retry(format!("{:?}", error)),
        };

    match object_exists {
        //
        // Internal implementation detail.
        false => match image_identity.kind {
            //
            // Internal implementation detail.
            ResourceKind::PageImage => {
                process_unverified_page_image(nucl, repo, image_identity).await
            }

            _ => process_missing_image(nucl, repo, image_identity).await,
        },

        true => {
            //
            // Internal implementation detail.
            if image_identity.kind == ResourceKind::PageImage {
                return process_existing_page_image(nucl, repo, image_identity)
                    .await;
            }

            process_existing_image(nucl, repo, image_identity).await
        }
    }
}

/// Deletes an image object from the storage backend.
/// Deletes an image object from the storage backend.
#[instrument(level = "info", skip_all)]
// Delete the storage object; returns retry task flow on failure.
async fn handle_delete<I>(image_pool: &I, object_key: &str) -> TaskFlow
where
    I: ImageManager + Send + Sync,
{
    match image_pool.delete_object(object_key).await {
        //
        // Internal implementation detail.
        Ok(()) => TaskFlow::Complete,

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}

/// Clears an unverified current page image without changing chapter workflow.
#[instrument(level = "info", skip_all)]
// For pages whose upload finished but the row was not persisted, only update the upload flag without triggering the image pipeline.
async fn process_unverified_page_image<N, R>(
    nucl: &N,
    repo: &R,
    image_identity: ImageIdentity<'_>,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError>,
    R: ChapterRepo<RdbContext> + PageRepo<RdbContext> + Send + Sync,
{
    let page_info = match repo
        .run(&GetPageInfo {
            id: image_identity.resource_id,
        })
        .await
    {
        // Internal implementation detail.
        Ok(page_info) => page_info,

        Err(BaseError::Expected { .. }) => return TaskFlow::Complete,

        Err(error) => return TaskFlow::Retry(format!("{:?}", error)),
    };

    match (
        page_info.image_version == image_identity.version,
        page_info.image_key.as_deref() == Some(image_identity.object_key),
    ) {
        //
        // Internal implementation detail.
        (false, _) => return TaskFlow::Complete,

        (true, false) => {
            return TaskFlow::Dead(
                "prom page image version matches but object key differs".into(),
            );
        }

        (true, true) => {}
    }

    let outcome: BaseResult<()> = nucl
        .coord(async move |context| {
            //
            // Internal implementation detail.
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
                .step(
                    context,
                    &GetPageInfoExcluded {
                        id: image_identity.resource_id,
                    },
                )
                .await?;

            if locked_page_info.image_version != image_identity.version
                || locked_page_info.image_key.as_deref()
                    != Some(image_identity.object_key)
            {
                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: "stale page image identity".into(),
                });
            }

            repo.step(
                context,
                &SetPageImageUploaded {
                    id: image_identity.resource_id,
                    image_version: image_identity.version,
                    image_key: image_identity.object_key,
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
        // Internal implementation detail.
        Ok(()) | Err(BaseError::Expected { .. }) => TaskFlow::Complete,

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}

#[instrument(level = "info", skip_all)]
// When the storage object does not exist, clean up the old database pointer only if the identity is still valid.
async fn process_missing_image<N, R>(
    nucl: &N,
    repo: &R,
    image_identity: ImageIdentity<'_>,
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
    match resource::mark_current_or_classify(nucl, repo, image_identity, false)
        .await
    {
        // Internal implementation detail.
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
// Compare the current page image identity and mark as uploaded when the object exists and is current.
async fn process_existing_page_image<N, R>(
    nucl: &N,
    repo: &R,
    image_identity: ImageIdentity<'_>,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError>,
    R: ChapterRepo<RdbContext> + PageRepo<RdbContext> + Send + Sync,
{
    let page_info = match repo
        .run(&GetPageInfo {
            id: image_identity.resource_id,
        })
        .await
    {
        // Internal implementation detail.
        Ok(page_info) => page_info,

        Err(BaseError::Expected { .. }) => {
            // SAFETY: stale payload keys are cleaned only by dedicated Delete tasks.
            return TaskFlow::Complete;
        }

        Err(error) => return TaskFlow::Retry(format!("{:?}", error)),
    };

    match (
        page_info.image_version == image_identity.version,
        page_info.image_key.as_deref() == Some(image_identity.object_key),
    ) {
        //
        // Internal implementation detail.
        (false, _) => return TaskFlow::Complete,

        (true, false) => {
            return TaskFlow::Dead(
                "prom page image version matches but object key differs".into(),
            );
        }

        (true, true) => {}
    }

    let outcome: BaseResult<()> = nucl
        .coord(async move |context| {
            //
            // Internal implementation detail.
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
                .step(
                    context,
                    &GetPageInfoExcluded {
                        id: image_identity.resource_id,
                    },
                )
                .await?;

            if locked_page_info.image_version != image_identity.version
                || locked_page_info.image_key.as_deref()
                    != Some(image_identity.object_key)
            {
                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: "stale page image identity".into(),
                });
            }

            repo.step(
                context,
                &MarkPageImageUploaded {
                    id: image_identity.resource_id,
                    image_version: image_identity.version,
                    image_key: Some(image_identity.object_key),
                },
            )
            .await?;

            accept(())
        })
        .await
        .map_err(Into::into);

    match outcome {
        //
        // Internal implementation detail.
        Ok(()) => TaskFlow::Complete,

        Err(BaseError::Expected { .. }) => TaskFlow::Complete,

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}

#[instrument(level = "info", skip_all)]
// Perform identity comparison for non-page resources; update upload status when identities match.
async fn process_existing_image<N, R>(
    nucl: &N,
    repo: &R,
    image_identity: ImageIdentity<'_>,
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
{
    let resource_state =
        resource::mark_current_or_classify(nucl, repo, image_identity, true)
            .await;

    match resource_state {
        //
        // Internal implementation detail.
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
