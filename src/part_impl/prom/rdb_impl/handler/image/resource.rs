//! Resource-specific image identity checks and updates.

use poprako_orchestra::Nucl;
use tracing::instrument;

use crate::part::prom::payload::image::ResourceKind;
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
use crate::part_impl::prom::rdb_impl::handler::image::identity::ImageIdentity;
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseRest, accept};

/// Classification of a deferred image payload against persisted identity.
pub enum ResourceState {
    /// The image version and object key match the current record.
    Current,

    /// The image version has been superseded.
    Stale,

    /// The referenced resource no longer exists.
    Missing,

    /// The version is current but another identity field differs.
    Mismatched,
}

// Current image identity read from a persisted resource record.
// Holds the latest DB version and object key for mismatch checks.
struct CurrentImageIdentity<'a> {
    //
    // Internal state field `version`.
    // Version number of the resource in the database, used to detect stale writes.
    version: u32,
    // Object key associated with the current database record.
    object_key: Option<&'a str>,
}

/// Classifies a payload under lock and applies its uploaded state when current.
#[instrument(level = "info", skip_all)]
pub async fn mark_current_or_classify<N, R>(
    nucl: &N,
    repo: &R,
    image_identity: ImageIdentity<'_>,
    image_uploaded: bool,
) -> BaseRest<ResourceState>
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
            // Internal implementation detail.
            let resource_state =
                classify_expected_mark(repo, context, image_identity).await?;

            if !matches!(resource_state, ResourceState::Current) {
                return accept(resource_state);
            }

            mark_uploaded_by_kind(
                repo,
                context,
                image_identity,
                image_uploaded,
            )
            .await?;

            accept(ResourceState::Current)
        })
        .await?;

    accept(resource_state)
}

// Internal implementation of `classify_expected_mark`.
async fn classify_expected_mark<R>(
    repo: &R,
    context: &mut RdbContext,
    image_identity: ImageIdentity<'_>,
) -> BaseRest<ResourceState>
where
    R: UserRepo<RdbContext>
        + TeamRepo<RdbContext>
        + ComicRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync,
{
    match image_identity.kind {
        //
        // Internal implementation detail.
        ResourceKind::UserAvatar => {
            match repo
                .step(
                    context,
                    &GetUserInfoExcluded::Id {
                        id: image_identity.resource_id,
                    },
                )
                .await
            {
                // Internal implementation detail.
                Ok(info) => {
                    //
                    // Internal implementation detail.
                    let current_identity = CurrentImageIdentity {
                        version: info.avatar_version,
                        object_key: info.avatar_key.as_deref(),
                    };

                    classify_current_identity(current_identity, image_identity)
                }

                Err(BaseError::Expected { .. }) => {
                    accept(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }

        ResourceKind::TeamAvatar => {
            match repo
                .step(
                    context,
                    &GetTeamInfoExcluded::Id {
                        id: image_identity.resource_id,
                    },
                )
                .await
            {
                // Internal implementation detail.
                Ok(info) => {
                    //
                    // Internal implementation detail.
                    let current_identity = CurrentImageIdentity {
                        version: info.avatar_version,
                        object_key: info.avatar_key.as_deref(),
                    };

                    classify_current_identity(current_identity, image_identity)
                }

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
                        id: image_identity.resource_id,
                        incls: &[],
                    },
                )
                .await
            {
                // Internal implementation detail.
                Ok(info) => {
                    //
                    // Internal implementation detail.
                    let current_identity = CurrentImageIdentity {
                        version: info.cover_version,
                        object_key: info.cover_key.as_deref(),
                    };

                    classify_current_identity(current_identity, image_identity)
                }

                Err(BaseError::Expected { .. }) => {
                    accept(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }

        ResourceKind::PageImage => {
            match repo
                .step(
                    context,
                    &GetPageInfoExcluded {
                        id: image_identity.resource_id,
                    },
                )
                .await
            {
                // Internal implementation detail.
                Ok(info) => {
                    //
                    // Internal implementation detail.
                    let current_identity = CurrentImageIdentity {
                        version: info.image_version,
                        object_key: info.image_key.as_deref(),
                    };

                    classify_current_identity(current_identity, image_identity)
                }

                Err(BaseError::Expected { .. }) => {
                    accept(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }
    }
}

// Internal implementation of `mark_uploaded_by_kind`.
async fn mark_uploaded_by_kind<R>(
    repo: &R,
    context: &mut RdbContext,
    image_identity: ImageIdentity<'_>,
    image_uploaded: bool,
) -> BaseRest<()>
where
    R: UserRepo<RdbContext>
        + TeamRepo<RdbContext>
        + ComicRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync,
{
    match image_identity.kind {
        //
        // Internal implementation detail.
        ResourceKind::UserAvatar => {
            repo.step(
                context,
                &UpdateUser::MarkAvatarUploaded {
                    id: image_identity.resource_id,
                    avatar_version: image_identity.version,
                    avatar_key: Some(image_identity.object_key),
                    avatar_uploaded: image_uploaded,
                },
            )
            .await
        }

        ResourceKind::TeamAvatar => {
            repo.step(
                context,
                &UpdateTeam::MarkAvatarUploaded {
                    id: image_identity.resource_id,
                    avatar_version: image_identity.version,
                    avatar_key: Some(image_identity.object_key),
                    avatar_uploaded: image_uploaded,
                },
            )
            .await
        }

        ResourceKind::ComicCover => {
            repo.step(
                context,
                &MarkComicCoverUploaded {
                    id: image_identity.resource_id,
                    cover_version: image_identity.version,
                    cover_key: Some(image_identity.object_key),
                    cover_uploaded: image_uploaded,
                },
            )
            .await
        }

        ResourceKind::PageImage => {
            repo.step(
                context,
                &MarkPageImageUploaded {
                    id: image_identity.resource_id,
                    image_version: image_identity.version,
                    image_key: Some(image_identity.object_key),
                },
            )
            .await
        }
    }
}

// Internal implementation of `classify_current_identity`.
fn classify_current_identity(
    current_identity: CurrentImageIdentity<'_>,
    image_identity: ImageIdentity<'_>,
) -> BaseRest<ResourceState> {
    match (
        current_identity.version == image_identity.version,
        current_identity.object_key == Some(image_identity.object_key),
    ) {
        //
        // Internal implementation detail.
        (false, _) => accept(ResourceState::Stale),

        (true, false) => accept(ResourceState::Mismatched),

        (true, true) => accept(ResourceState::Current),
    }
}
