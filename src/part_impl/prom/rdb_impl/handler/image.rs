//! Handler for the "image" prom topic.
//!
//! Dispatches image [`Payload`] variants to their concrete implementations.

use poprako_orchestra::Nucl;
use tracing::{Level, instrument};

use crate::part::image::ImageManager;
use crate::part::prom::payload::image::{Payload, ResourceKind};
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::comic::{
    GetComicInfoExcluded, MarkComicCoverUploaded,
};
use crate::part::repo::oper::page::{
    GetPageInfoExcluded, MarkPageImageUploaded,
};
use crate::part::repo::oper::team::{GetTeamInfoExcluded, UpdateTeam};
use crate::part::repo::oper::user::{GetUserInfoExcluded, UpdateUser};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::part_impl::prom::rdb_impl::handler::TaskFlow;
use crate::part_impl::shared::RdbContext;
use crate::result::{RegularError, RegularResult};

enum ResourceState {
    Current,
    Stale,
    Missing,
}

fn classify_current_version(
    current_version: u32,
    image_version: u32,
    error_message: &'static str,
) -> RegularResult<ResourceState> {
    match current_version == image_version {
        //
        true => Err(RegularError::Unrecoverable {
            message: error_message.into(),
        }),

        false => Ok(ResourceState::Stale),
    }
}

/// Dispatch an image [`Payload`] to its concrete handler.
pub async fn handle<N, R, I>(
    nucl: &N,
    repo: &R,
    image_pool: &I,
    task: &Payload,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = RegularError>,
    R: UserRepo<RdbContext>
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
        } => {
            handle_check_uploaded(
                nucl,
                repo,
                image_pool,
                *resource_kind,
                resource_id,
                object_key,
                *version,
            )
            .await
        }

        Payload::Delete { object_key } => {
            handle_delete(image_pool, object_key).await
        }
    }
}

/// Verifies that an uploaded image object exists and confirms current DB ownership.
#[instrument(skip(nucl, repo, image_pool), level = Level::DEBUG)]
async fn handle_check_uploaded<N, R, I>(
    nucl: &N,
    repo: &R,
    image_pool: &I,
    kind: ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = RegularError>,
    R: UserRepo<RdbContext>
        + TeamRepo<RdbContext>
        + ComicRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync,
    I: ImageManager + Send + Sync,
{
    let exists = match image_pool.head_object(object_key).await {
        //
        Ok(exists) => exists,

        Err(error) => return TaskFlow::Retry(format!("{:?}", error)),
    };

    match exists {
        //
        false => TaskFlow::Complete,

        true => {
            process_existing_image(
                nucl,
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

async fn process_existing_image<N, R, I>(
    nucl: &N,
    repo: &R,
    image_pool: &I,
    kind: ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = RegularError>,
    R: UserRepo<RdbContext>
        + TeamRepo<RdbContext>
        + ComicRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync,
    I: ImageManager + Send + Sync,
{
    let resource_state =
        mark_current_or_classify(nucl, repo, kind, resource_id, image_version)
            .await;

    match resource_state {
        //
        Ok(ResourceState::Current) | Ok(ResourceState::Stale) => {
            TaskFlow::Complete
        }

        Ok(ResourceState::Missing) => {
            handle_delete(image_pool, object_key).await
        }

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}

async fn mark_current_or_classify<N, R>(
    nucl: &N,
    repo: &R,
    kind: ResourceKind,
    resource_id: &str,
    image_version: u32,
) -> RegularResult<ResourceState>
where
    N: Nucl<Context = RdbContext, Error = RegularError>,
    R: UserRepo<RdbContext>
        + TeamRepo<RdbContext>
        + ComicRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync,
{
    let resource_state = nucl
        .coord(async move |context| {
            match mark_uploaded_by_kind(
                repo,
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
                        repo,
                        context,
                        kind,
                        resource_id,
                        image_version,
                    )
                    .await
                }

                Err(error) => Err(error),
            }
        })
        .await?;

    Ok(resource_state)
}

async fn mark_uploaded_by_kind<R>(
    repo: &R,
    context: &mut RdbContext,
    kind: ResourceKind,
    resource_id: &str,
    image_version: u32,
) -> RegularResult<()>
where
    R: UserRepo<RdbContext>
        + TeamRepo<RdbContext>
        + ComicRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync,
{
    match kind {
        //
        ResourceKind::UserAvatar => {
            //

            repo.step(
                context,
                &UpdateUser::MarkAvatarUploaded {
                    id: resource_id,
                    avatar_version: image_version,
                },
            )
            .await
        }

        ResourceKind::TeamAvatar => {
            //

            repo.step(
                context,
                &UpdateTeam::MarkAvatarUploaded {
                    id: resource_id,
                    avatar_version: image_version,
                },
            )
            .await
        }

        ResourceKind::ComicCover => {
            repo.step(
                context,
                &MarkComicCoverUploaded {
                    id: resource_id,
                    cover_version: image_version,
                },
            )
            .await
        }

        ResourceKind::PageImage => {
            repo.step(
                context,
                &MarkPageImageUploaded {
                    id: resource_id,
                    image_version,
                },
            )
            .await
        }
    }
}

async fn classify_expected_mark<R>(
    repo: &R,
    context: &mut RdbContext,
    kind: ResourceKind,
    resource_id: &str,
    image_version: u32,
) -> RegularResult<ResourceState>
where
    R: UserRepo<RdbContext>
        + TeamRepo<RdbContext>
        + ComicRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync,
{
    match kind {
        //
        ResourceKind::UserAvatar => {
            //

            match repo
                .step(context, &GetUserInfoExcluded::Id { id: resource_id })
                .await
            {
                //
                Ok(user_info) => classify_current_version(
                    user_info.avatar_version,
                    image_version,
                    "[RdbPromImageHandler::classify_expected_mark] current user avatar version failed to mark uploaded",
                ),

                Err(RegularError::Expected { .. }) => {
                    Ok(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }

        ResourceKind::TeamAvatar => {
            //

            match repo
                .step(context, &GetTeamInfoExcluded::Id { id: resource_id })
                .await
            {
                //
                Ok(team_info) => classify_current_version(
                    team_info.avatar_version,
                    image_version,
                    "[RdbPromImageHandler::classify_expected_mark] current team avatar version failed to mark uploaded",
                ),

                Err(RegularError::Expected { .. }) => {
                    Ok(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }

        ResourceKind::ComicCover => {
            match repo
                .step(
                    context,
                    &GetComicInfoExcluded {
                        id: resource_id,
                        incls: &[],
                    },
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

                Err(error) => Err(error),
            }
        }

        ResourceKind::PageImage => {
            match repo
                .step(context, &GetPageInfoExcluded { id: resource_id })
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

                Err(error) => Err(error),
            }
        }
    }
}

/// Deletes an image object from the storage backend.
#[instrument(skip(image_pool), level = Level::DEBUG)]
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
