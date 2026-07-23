//! Handler for the "image" prom topic.
//!
//! Dispatches image [`Payload`] variants to their concrete implementations.

use poprako_orchestra::Nucl;
use tracing::instrument;

use crate::part::image::{ImageManager, ImageObjectInfo};
use crate::part::prom::payload::image::{Payload, ResourceKind};
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::chapter::{
    CompleteChapterRawProvide, ResetChapterRawProvide,
};
use crate::part::repo::oper::comic::{
    GetComicInfoExcluded, MarkComicCoverUploaded,
};
use crate::part::repo::oper::page::{
    GetPageInfo, GetPageInfoExcluded, MarkPageImageUploaded,
    SetPageImageUploaded,
};
use crate::part::repo::oper::team::{GetTeamInfoExcluded, UpdateTeam};
use crate::part::repo::oper::user::{GetUserInfoExcluded, UpdateUser};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::part_impl::prom::rdb_impl::handler::task_flow::TaskFlow;
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult, accept};

enum ResourceState {
    /// The image version matches the current DB record.
    Current,
    /// The image version is outdated; the resource has been superseded.
    Stale,
    /// The referenced resource no longer exists.
    Missing,
    /// The version is current but the persisted object key differs.
    Mismatched,
}

fn classify_current_identity(
    current_version: u32,
    current_object_key: Option<&str>,
    image_version: u32,
    object_key: &str,
    err_message: &'static str,
) -> BaseResult<ResourceState> {
    match (
        current_version == image_version,
        current_object_key == Some(object_key),
    ) {
        //
        (false, _) => accept(ResourceState::Stale),

        (true, false) => accept(ResourceState::Mismatched),

        (true, true) => Err(BaseError::Unrecoverable {
            message: err_message.into(),
        }),
    }
}

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
#[instrument(level = "info", skip_all)]
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
                )
                .await
            }

            _ => TaskFlow::Complete,
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
            )
            .await
        }
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
            return handle_delete(image_pool, object_key).await;
        }

        Err(error) => return TaskFlow::Retry(format!("{:?}", error)),
    };

    match (
        page_info.image_version == image_version,
        page_info.image_key.as_deref() == Some(object_key),
    ) {
        //
        (false, _) => return TaskFlow::Complete,

        (true, false) => return handle_delete(image_pool, object_key).await,

        (true, true) => {}
    }

    if page_info.image_byte_length != object_info.byte_length
        || page_info.image_hash != object_info.checksum_sha256
    {
        let verification_outcome = process_unverified_page_image(
            nucl,
            repo,
            resource_id,
            object_key,
            image_version,
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

    let outcome: BaseResult<bool> = nucl
        .coord(async move |context| {
            //
            repo.step(
                context,
                &MarkPageImageUploaded {
                    id: resource_id,
                    image_version,
                    image_key: Some(object_key),
                },
            )
            .await?;

            repo.step(
                context,
                &CompleteChapterRawProvide {
                    id: &page_info.chapter_id,
                },
            )
            .await?;

            accept(true)
        })
        .await
        .map_err(Into::into);

    match outcome {
        //
        Ok(_) => TaskFlow::Complete,

        Err(BaseError::Expected { .. }) => TaskFlow::Complete,

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}

/// Clears an unverified current page image and returns raw provision to pending.
#[instrument(level = "info", skip_all)]
async fn process_unverified_page_image<N, R>(
    nucl: &N,
    repo: &R,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
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

    if page_info.image_version != image_version
        || page_info.image_key.as_deref() != Some(object_key)
    {
        return TaskFlow::Complete;
    }

    let outcome: BaseResult<()> = nucl
        .coord(async move |context| {
            //
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

            repo.step(
                context,
                &ResetChapterRawProvide {
                    id: &page_info.chapter_id,
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
    let resource_state = mark_current_or_classify(
        nucl,
        repo,
        kind,
        resource_id,
        object_key,
        image_version,
    )
    .await;

    match resource_state {
        //
        Ok(ResourceState::Current) | Ok(ResourceState::Stale) => {
            TaskFlow::Complete
        }

        Ok(ResourceState::Missing) => {
            handle_delete(image_pool, object_key).await
        }

        Ok(ResourceState::Mismatched) => TaskFlow::Dead(
            "prom image identity does not match current resource".into(),
        ),

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}

#[instrument(level = "info", skip_all)]
async fn mark_current_or_classify<N, R>(
    nucl: &N,
    repo: &R,
    kind: ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
) -> BaseResult<ResourceState>
where
    N: Nucl<Context = RdbContext, Error = BaseError>,
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
                object_key,
                image_version,
            )
            .await
            {
                Ok(()) => accept(ResourceState::Current),

                Err(BaseError::Expected { .. }) => {
                    classify_expected_mark(
                        repo,
                        context,
                        kind,
                        resource_id,
                        object_key,
                        image_version,
                    )
                    .await
                }

                Err(error) => Err(error),
            }
        })
        .await?;

    accept(resource_state)
}

#[instrument(level = "info", skip_all)]
async fn mark_uploaded_by_kind<R>(
    repo: &R,
    context: &mut RdbContext,
    kind: ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
) -> BaseResult<()>
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
            repo.step(
                context,
                &UpdateUser::MarkAvatarUploaded {
                    id: resource_id,
                    avatar_version: image_version,
                    avatar_key: Some(object_key),
                },
            )
            .await
        }

        ResourceKind::TeamAvatar => {
            repo.step(
                context,
                &UpdateTeam::MarkAvatarUploaded {
                    id: resource_id,
                    avatar_version: image_version,
                    avatar_key: Some(object_key),
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
                    cover_key: Some(object_key),
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
                    image_key: Some(object_key),
                },
            )
            .await
        }
    }
}

#[instrument(level = "info", skip_all)]
async fn classify_expected_mark<R>(
    repo: &R,
    context: &mut RdbContext,
    kind: ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
) -> BaseResult<ResourceState>
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
            match repo
                .step(context, &GetUserInfoExcluded::Id { id: resource_id })
                .await
            {
                //
                Ok(user_info) => classify_current_identity(
                    user_info.avatar_version,
                    user_info.avatar_key.as_deref(),
                    image_version,
                    object_key,
                    "[RdbPromImageHandler::classify_expected_mark] current user avatar version failed to mark uploaded",
                ),

                Err(BaseError::Expected { .. }) => {
                    accept(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }

        ResourceKind::TeamAvatar => {
            match repo
                .step(context, &GetTeamInfoExcluded::Id { id: resource_id })
                .await
            {
                //
                Ok(team_info) => classify_current_identity(
                    team_info.avatar_version,
                    team_info.avatar_key.as_deref(),
                    image_version,
                    object_key,
                    "[RdbPromImageHandler::classify_expected_mark] current team avatar version failed to mark uploaded",
                ),

                Err(BaseError::Expected { .. }) => {
                    accept(ResourceState::Missing)
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
                Ok(comic_info) => classify_current_identity(
                    comic_info.cover_version,
                    comic_info.cover_key.as_deref(),
                    image_version,
                    object_key,
                    "[RdbPromImageHandler::classify_expected_mark] current comic cover version failed to mark uploaded",
                ),

                Err(BaseError::Expected { .. }) => {
                    accept(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }

        ResourceKind::PageImage => {
            match repo
                .step(context, &GetPageInfoExcluded { id: resource_id })
                .await
            {
                Ok(page_info) => classify_current_identity(
                    page_info.image_version,
                    page_info.image_key.as_deref(),
                    image_version,
                    object_key,
                    "[RdbPromImageHandler::classify_expected_mark] current page image version failed to mark uploaded",
                ),

                Err(BaseError::Expected { .. }) => {
                    accept(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }
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
