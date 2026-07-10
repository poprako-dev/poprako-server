//! Handler for the "image" prom topic.
//!
//! Dispatches [`ImageTask`] variants to their concrete implementations.

use std::sync::Arc;

use tracing::{Level, instrument};

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::part::image::ImagePool;
use crate::part::prom::task::{ImageKind, ImageTask};
use crate::part_impl::prom::rdb_impl::repo::LocalMessageRepo;
use crate::part::repo::comic::ComicRepoTransactional;
use crate::part::repo::page::PageRepoTransactional;
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::step::page::PageStep;
use crate::part::repo::step::team::TeamStep;
use crate::part::repo::step::user::UserStep;
use crate::part::repo::team::TeamRepoTransactional;
use crate::part::repo::user::UserRepoTransactional;
use crate::part_impl::prom::rdb_impl::handler::TaskOutcome;
use crate::part_impl::shared::RdbContext;
use crate::result::{RegularError, RegularResult};
use crate::util::DeriveTransactional;

enum ResourceState {
    Current,
    Stale,
    Missing,
}

fn classify_current_version(
    current_version: i64,
    image_version: i64,
    error_message: &'static str,
) -> RegularResult<ResourceState> {
    match current_version == image_version {
        true => Err(RegularError::Unrecoverable {
            message: error_message.into(),
        }),
        false => Ok(ResourceState::Stale),
    }
}

/// Dispatch an [`ImageTask`] to its concrete handler.
pub async fn handle<D, R, I>(
    drive: &D,
    repo: &Arc<R>,
    _local_message_repo: &LocalMessageRepo,
    image_pool: &I,
    task: &ImageTask<'_>,
) -> TaskOutcome
where
    D: Drive<RdbContext>,
    D::Error: Into<RegularError>,
    R: DeriveTransactional + Send + Sync,
    <R as DeriveTransactional>::Transactional: UserRepoTransactional<RdbContext>
        + TeamRepoTransactional<RdbContext>
        + ComicRepoTransactional<RdbContext>
        + PageRepoTransactional<RdbContext>
        + Send
        + Sync,
    I: ImagePool + Send + Sync,
{
    match task {
        ImageTask::CheckUploaded {
            kind,
            resource_id,
            object_key,
            image_version,
        } => {
            handle_check_uploaded(
                drive,
                repo,
                image_pool,
                *kind,
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

/// Verifies that an uploaded image object exists and confirms current DB ownership.
#[instrument(skip(drive, repo, image_pool), level = Level::DEBUG)]
async fn handle_check_uploaded<D, R, I>(
    drive: &D,
    repo: &Arc<R>,
    image_pool: &I,
    kind: ImageKind,
    resource_id: &str,
    object_key: &str,
    image_version: i64,
) -> TaskOutcome
where
    D: Drive<RdbContext>,
    D::Error: Into<RegularError>,
    R: DeriveTransactional + Send + Sync,
    <R as DeriveTransactional>::Transactional: UserRepoTransactional<RdbContext>
        + TeamRepoTransactional<RdbContext>
        + ComicRepoTransactional<RdbContext>
        + PageRepoTransactional<RdbContext>
        + Send
        + Sync,
    I: ImagePool + Send + Sync,
{
    let exists = match image_pool.head_object(object_key).await {
        Ok(exists) => exists,
        Err(e) => return TaskOutcome::Retry(format!("{:?}", e)),
    };

    match exists {
        false => TaskOutcome::Complete,
        true => {
            process_existing_image(
                drive,
                repo,
                image_pool,
                kind,
                resource_id,
                object_key,
                image_version,
            )
            .await
        }
    }
}

async fn process_existing_image<D, R, I>(
    drive: &D,
    repo: &Arc<R>,
    image_pool: &I,
    kind: ImageKind,
    resource_id: &str,
    object_key: &str,
    image_version: i64,
) -> TaskOutcome
where
    D: Drive<RdbContext>,
    D::Error: Into<RegularError>,
    R: DeriveTransactional + Send + Sync,
    <R as DeriveTransactional>::Transactional: UserRepoTransactional<RdbContext>
        + TeamRepoTransactional<RdbContext>
        + ComicRepoTransactional<RdbContext>
        + PageRepoTransactional<RdbContext>
        + Send
        + Sync,
    I: ImagePool + Send + Sync,
{
    let resource_state =
        mark_current_or_classify(drive, repo, kind, resource_id, image_version)
            .await;

    match resource_state {
        Ok(ResourceState::Current) => TaskOutcome::Complete,
        Ok(ResourceState::Stale) => TaskOutcome::Complete,
        Ok(ResourceState::Missing) => {
            handle_delete(image_pool, object_key).await
        }
        Err(e) => TaskOutcome::Retry(format!("{:?}", e)),
    }
}

async fn mark_current_or_classify<D, R>(
    drive: &D,
    repo: &Arc<R>,
    kind: ImageKind,
    resource_id: &str,
    image_version: i64,
) -> RegularResult<ResourceState>
where
    D: Drive<RdbContext>,
    D::Error: Into<RegularError>,
    R: DeriveTransactional + Send + Sync,
    <R as DeriveTransactional>::Transactional: UserRepoTransactional<RdbContext>
        + TeamRepoTransactional<RdbContext>
        + ComicRepoTransactional<RdbContext>
        + PageRepoTransactional<RdbContext>
        + Send
        + Sync,
{
    let repo = Arc::clone(repo);

    drive
        .with_context(async move |context| {
            let transactional = repo.derive_transactional().await;

            match mark_uploaded_by_kind(
                &transactional,
                context,
                kind,
                resource_id,
                image_version,
            )
            .await
            {
                Ok(()) => Ok(ResourceState::Current),
                Err(RegularError::Expected { .. }) => {
                    classify_expected_mark(
                        &transactional,
                        context,
                        kind,
                        resource_id,
                        image_version,
                    )
                    .await
                }
                Err(e) => Err(e),
            }
        })
        .await
        .map_err(|e| e.into())
}

async fn mark_uploaded_by_kind<T>(
    transactional: &T,
    context: &mut RdbContext,
    kind: ImageKind,
    resource_id: &str,
    image_version: i64,
) -> RegularResult<()>
where
    T: UserRepoTransactional<RdbContext>
        + TeamRepoTransactional<RdbContext>
        + ComicRepoTransactional<RdbContext>
        + PageRepoTransactional<RdbContext>
        + Send
        + Sync,
{
    match kind {
        ImageKind::UserAvatar => {
            Advance::advance(
                transactional,
                context,
                &UserStep::mark_avatar_uploaded(resource_id, image_version),
            )
            .await
        }
        ImageKind::TeamAvatar => {
            Advance::advance(
                transactional,
                context,
                &TeamStep::mark_avatar_uploaded(resource_id, image_version),
            )
            .await
        }
        ImageKind::ComicCover => {
            Advance::advance(
                transactional,
                context,
                &ComicStep::mark_cover_uploaded(resource_id, image_version),
            )
            .await
        }
        ImageKind::PageImage => {
            Advance::advance(
                transactional,
                context,
                &PageStep::mark_image_uploaded(resource_id, image_version),
            )
            .await
        }
    }
}

async fn classify_expected_mark<T>(
    transactional: &T,
    context: &mut RdbContext,
    kind: ImageKind,
    resource_id: &str,
    image_version: i64,
) -> RegularResult<ResourceState>
where
    T: UserRepoTransactional<RdbContext>
        + TeamRepoTransactional<RdbContext>
        + ComicRepoTransactional<RdbContext>
        + PageRepoTransactional<RdbContext>
        + Send
        + Sync,
{
    match kind {
        ImageKind::UserAvatar => {
            match Advance::advance(
                transactional,
                context,
                &UserStep::get_info_excluded(resource_id),
            )
            .await
            {
                Ok(user_info) => classify_current_version(
                    user_info.avatar_version,
                    image_version,
                    "[RdbPromImageHandler::classify_expected_mark] current user avatar version failed to mark uploaded",
                ),
                Err(RegularError::Expected { .. }) => {
                    Ok(ResourceState::Missing)
                }
                Err(e) => Err(e),
            }
        }
        ImageKind::TeamAvatar => {
            match Advance::advance(
                transactional,
                context,
                &TeamStep::get_info_excluded(resource_id),
            )
            .await
            {
                Ok(team_info) => classify_current_version(
                    team_info.avatar_version,
                    image_version,
                    "[RdbPromImageHandler::classify_expected_mark] current team avatar version failed to mark uploaded",
                ),
                Err(RegularError::Expected { .. }) => {
                    Ok(ResourceState::Missing)
                }
                Err(e) => Err(e),
            }
        }
        ImageKind::ComicCover => {
            match Advance::advance(
                transactional,
                context,
                &ComicStep::get_info_excluded(resource_id, &[]),
            )
            .await
            {
                Ok(comic_info) => classify_current_version(
                    comic_info.cover_version,
                    image_version,
                    "[RdbPromImageHandler::classify_expected_mark] current comic cover version failed to mark uploaded",
                ),
                Err(RegularError::Expected { .. }) => {
                    Ok(ResourceState::Missing)
                }
                Err(e) => Err(e),
            }
        }
        ImageKind::PageImage => {
            match Advance::advance(
                transactional,
                context,
                &PageStep::get_info_excluded(resource_id),
            )
            .await
            {
                Ok(page_info) => classify_current_version(
                    page_info.image_version,
                    image_version,
                    "[RdbPromImageHandler::classify_expected_mark] current page image version failed to mark uploaded",
                ),
                Err(RegularError::Expected { .. }) => {
                    Ok(ResourceState::Missing)
                }
                Err(e) => Err(e),
            }
        }
    }
}

/// Deletes an image object from the storage backend.
#[instrument(skip(image_pool), level = Level::DEBUG)]
async fn handle_delete<I>(image_pool: &I, object_key: &str) -> TaskOutcome
where
    I: ImagePool + Send + Sync,
{
    match image_pool.delete_object(object_key).await {
        Ok(()) => TaskOutcome::Complete,
        Err(e) => TaskOutcome::Retry(format!("{:?}", e)),
    }
}
