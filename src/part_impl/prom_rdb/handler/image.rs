//! Handler for the "image" prom topic.
//!
//! Dispatches [`ImageTask`] variants to their concrete implementations.

use std::sync::Arc;

use tracing::{Level, instrument};

use crate::part::image::ImagePool;
use crate::part::prom::task::{ImageKind, ImageTask};
use crate::result::RegularResult;

/// Dispatch an [`ImageTask`] to its concrete handler.
pub(crate) async fn handle<D, R, P, I>(
    _drive: &D,
    _repo: &Arc<R>,
    _prom: &P,
    image_pool: &I,
    task: &ImageTask<'_>,
) -> RegularResult<()>
where
    I: ImagePool + Send + Sync,
{
    match task {
        //
        ImageTask::CheckUploaded {
            kind,
            resource_id,
            object_key,
            image_version,
        } => {
            handle_check_uploaded(
                image_pool,
                kind,
                resource_id,
                object_key,
                *image_version,
            )
            .await
        }

        ImageTask::Delete { object_key } => {
            handle_delete(image_pool, object_key).await
        }
    }
}

#[instrument(skip(image_pool), level = Level::DEBUG)]
async fn handle_check_uploaded<I>(
    image_pool: &I,
    kind: &ImageKind,
    resource_id: &str,
    object_key: &str,
    image_version: i64,
) -> RegularResult<()>
where
    I: ImagePool + Send + Sync,
{
    let exists = image_pool.head_object(object_key).await?;

    if exists {
        tracing::info!(
            kind = ?kind,
            resource_id = %resource_id,
            object_key = %object_key,
            image_version = %image_version,
            "[handle_check_uploaded] object exists, DB update deferred",
        );
    } else {
        tracing::warn!(
            kind = ?kind,
            resource_id = %resource_id,
            object_key = %object_key,
            "[handle_check_uploaded] object not found in storage",
        );
    }

    Ok(())
}

#[instrument(skip(image_pool), level = Level::DEBUG)]
async fn handle_delete<I>(image_pool: &I, object_key: &str) -> RegularResult<()>
where
    I: ImagePool + Send + Sync,
{
    image_pool.delete_object(object_key).await
}
