//! Handler for the "image" prom topic.
//!
//! Dispatches image [`ImagePayload`] variants to their concrete implementations.

// Internal organization of the `identity` module.
mod identity;
// Internal organization of the `resource` module.
mod resource;

use poprako_orchestra::{Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use crate::model::write::page::PageImageRepl;
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
use crate::result::{BaseError, ExpectedVariant, accept};
use crate::shared::RdbContext;

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

            Err(error) => {
                //
                return TaskFlow::Retry {
                    err_message: format!("{:?}", error),
                };
            }
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
                //
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

        Err(error) => TaskFlow::Retry {
            err_message: format!("{:?}", error),
        },
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
    let page_info = match (GetPageInfo {
        id: image_identity.resource_id,
    })
    .run_on(repo)
    .await
    {
        // Internal implementation detail.
        Ok(page_info) => page_info,

        Err(BaseError::Expected { .. }) => return TaskFlow::Complete,

        Err(error) => {
            //
            return TaskFlow::Retry {
                err_message: format!("{:?}", error),
            };
        }
    };

    match (
        page_info.image_version == Some(image_identity.version),
        page_info.image_key.as_deref() == Some(image_identity.object_key),
    ) {
        //
        // Internal implementation detail.
        (false, _) => return TaskFlow::Complete,

        (true, false) => {
            //
            return TaskFlow::Dead {
                err_message:
                    "prom page image version matches but object key differs"
                        .into(),
            };
        }

        (true, true) => {}
    }

    let outcome = nucl
        .coord(async move |context| {
            //
            // Internal implementation detail.
            // NOTE: Chapter -> Page is the shared lock order that prevents
            // both deadlocks and chapter upload-summary races.
            GetChapterInfoExcluded {
                id: &page_info.chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            let locked_page_info = GetPageInfoExcluded {
                id: image_identity.resource_id,
            }
            .step_on(repo, context)
            .await?;

            let image_version_matches =
                locked_page_info.image_version == Some(image_identity.version);

            let image_key_matches = locked_page_info.image_key.as_deref()
                == Some(image_identity.object_key);

            match (image_version_matches, image_key_matches) {
                //
                (true, true) => {}

                (false, _) | (true, false) => {
                    //
                    let err_message = "stale page image identity";

                    tracing::warn!(
                        err_variant = ?ExpectedVariant::Args,
                        err_message = %err_message,
                        resource_kind = ?image_identity.kind,
                        resource_id = %image_identity.resource_id,
                        image_version = image_identity.version,
                        stored_image_version = locked_page_info.image_version,
                        image_key_present = locked_page_info.image_key.is_some(),
                        image_key_matches,
                        object_key_present = !image_identity.object_key.is_empty(),
                        operation = "process_unverified_page_image",
                        "expected error: stale page image identity",
                    );

                    return Err(BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: err_message.into(),
                    });
                }
            }

            let repl = PageImageRepl {
                id: image_identity.resource_id.to_owned(),
                image_version: image_identity.version,
                image_key: Some(image_identity.object_key.to_owned()),
                is_image_uploaded: false,
            };

            SetPageImageUploaded { repl: &repl }
                .step_on(repo, context)
                .await?;

            accept(())
        })
        .await
        .map_err(Into::into);

    match outcome {
        //
        // Internal implementation detail.
        Ok(()) | Err(BaseError::Expected { .. }) => TaskFlow::Complete,

        Err(error) => TaskFlow::Retry {
            err_message: format!("{:?}", error),
        },
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

        Ok(ResourceState::Mismatched) => TaskFlow::Dead {
            err_message: "prom image identity does not match current resource"
                .into(),
        },

        Err(error) => TaskFlow::Retry {
            err_message: format!("{:?}", error),
        },
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
    let page_info = match (GetPageInfo {
        id: image_identity.resource_id,
    })
    .run_on(repo)
    .await
    {
        // Internal implementation detail.
        Ok(page_info) => page_info,

        Err(BaseError::Expected { .. }) => {
            // SAFETY: stale payload keys are cleaned only by dedicated Delete tasks.
            return TaskFlow::Complete;
        }

        Err(error) => {
            //
            return TaskFlow::Retry {
                err_message: format!("{:?}", error),
            };
        }
    };

    match (
        page_info.image_version == Some(image_identity.version),
        page_info.image_key.as_deref() == Some(image_identity.object_key),
    ) {
        //
        // Internal implementation detail.
        (false, _) => return TaskFlow::Complete,

        (true, false) => {
            //
            return TaskFlow::Dead {
                err_message:
                    "prom page image version matches but object key differs"
                        .into(),
            };
        }

        (true, true) => {}
    }

    let outcome = nucl
        .coord(async move |context| {
            //
            // Internal implementation detail.
            // NOTE: Chapter -> Page is the shared lock order that prevents
            // both deadlocks and chapter upload-summary races.
            GetChapterInfoExcluded {
                id: &page_info.chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            let locked_page_info = GetPageInfoExcluded {
                id: image_identity.resource_id,
            }
            .step_on(repo, context)
            .await?;

            let image_version_matches =
                locked_page_info.image_version == Some(image_identity.version);

            let image_key_matches = locked_page_info.image_key.as_deref()
                == Some(image_identity.object_key);

            match (image_version_matches, image_key_matches) {
                //
                (true, true) => {}

                (false, _) | (true, false) => {
                    //
                    let err_message = "stale page image identity";

                    tracing::warn!(
                        err_variant = ?ExpectedVariant::Args,
                        err_message = %err_message,
                        resource_kind = ?image_identity.kind,
                        resource_id = %image_identity.resource_id,
                        image_version = image_identity.version,
                        stored_image_version = locked_page_info.image_version,
                        image_key_present = locked_page_info.image_key.is_some(),
                        image_key_matches,
                        object_key_present = !image_identity.object_key.is_empty(),
                        operation = "process_existing_page_image",
                        "expected error: stale page image identity",
                    );

                    return Err(BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: err_message.into(),
                    });
                }
            }

            let repl = PageImageRepl {
                id: image_identity.resource_id.to_owned(),
                image_version: image_identity.version,
                image_key: Some(image_identity.object_key.to_owned()),
                is_image_uploaded: true,
            };

            MarkPageImageUploaded { repl: &repl }
                .step_on(repo, context)
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

        Err(error) => TaskFlow::Retry {
            err_message: format!("{:?}", error),
        },
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

        Ok(ResourceState::Mismatched) => TaskFlow::Dead {
            err_message: "prom image identity does not match current resource"
                .into(),
        },

        Err(error) => TaskFlow::Retry {
            err_message: format!("{:?}", error),
        },
    }
}
