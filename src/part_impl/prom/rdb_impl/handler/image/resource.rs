//! Resource-specific image identity checks and updates.

use poprako_orchestra::Nucl;
use tracing::instrument;

use crate::part::prom::payload::image::ResourceKind;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::comic::{GetComicInfoExcluded, MarkComicCoverUploaded};
use crate::part::repo::oper::page::{GetPageInfoExcluded, MarkPageImageUploaded};
use crate::part::repo::oper::team::{GetTeamInfoExcluded, UpdateTeam};
use crate::part::repo::oper::user::{GetUserInfoExcluded, UpdateUser};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult, accept};
use crate::value::image::{ImageExt, ImageHash};

/// Classification of a deferred image payload against persisted identity.
pub enum ResourceState {
    /// The image version and full identity match the current record.
    Current,
    /// The image version has been superseded.
    Stale,
    /// The referenced resource no longer exists.
    Missing,
    /// The version is current but another identity field differs.
    Mismatched,
}

fn classify_current_identity(
    current_version: u32,
    current_object_key: Option<&str>,
    image_version: u32,
    object_key: &str,
    current_hash: &ImageHash,
    current_ext: ImageExt,
    image_hash: &ImageHash,
    image_ext: ImageExt,
) -> BaseResult<ResourceState> {
    match (
        current_version == image_version,
        current_object_key == Some(object_key)
            && current_hash == image_hash
            && current_ext == image_ext,
    ) {
        //
        (false, _) => accept(ResourceState::Stale),

        (true, false) => accept(ResourceState::Mismatched),

        (true, true) => accept(ResourceState::Current),
    }
}

/// Classifies a payload under lock and applies its uploaded state when current.
#[instrument(level = "info", skip_all)]
pub async fn mark_current_or_classify<N, R>(
    nucl: &N,
    repo: &R,
    kind: ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
    image_hash: &ImageHash,
    image_ext: ImageExt,
    image_uploaded: bool,
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
            //
            let resource_state = classify_expected_mark(
                repo,
                context,
                kind,
                resource_id,
                object_key,
                image_version,
                image_hash,
                image_ext,
            )
            .await?;

            if !matches!(resource_state, ResourceState::Current) {
                return accept(resource_state);
            }

            mark_uploaded_by_kind(
                repo,
                context,
                kind,
                resource_id,
                object_key,
                image_version,
                image_uploaded,
            )
            .await?;

            accept(ResourceState::Current)
        })
        .await?;

    accept(resource_state)
}

async fn mark_uploaded_by_kind<R>(
    repo: &R,
    context: &mut RdbContext,
    kind: ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
    image_uploaded: bool,
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
                    avatar_uploaded: image_uploaded,
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
                    avatar_uploaded: image_uploaded,
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
                    cover_uploaded: image_uploaded,
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

async fn classify_expected_mark<R>(
    repo: &R,
    context: &mut RdbContext,
    kind: ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
    image_hash: &ImageHash,
    image_ext: ImageExt,
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
                Ok(info) => classify_current_identity(
                    info.avatar_version,
                    info.avatar_key.as_deref(),
                    image_version,
                    object_key,
                    &info.avatar_hash,
                    info.avatar_ext,
                    image_hash,
                    image_ext,
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
                Ok(info) => classify_current_identity(
                    info.avatar_version,
                    info.avatar_key.as_deref(),
                    image_version,
                    object_key,
                    &info.avatar_hash,
                    info.avatar_ext,
                    image_hash,
                    image_ext,
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
                //
                Ok(info) => classify_current_identity(
                    info.cover_version,
                    info.cover_key.as_deref(),
                    image_version,
                    object_key,
                    &info.cover_hash,
                    info.cover_ext,
                    image_hash,
                    image_ext,
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
                //
                Ok(info) => classify_current_identity(
                    info.image_version,
                    info.image_key.as_deref(),
                    image_version,
                    object_key,
                    &info.image_hash,
                    info.image_ext,
                    image_hash,
                    image_ext,
                ),

                Err(BaseError::Expected { .. }) => {
                    accept(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }
    }
}
