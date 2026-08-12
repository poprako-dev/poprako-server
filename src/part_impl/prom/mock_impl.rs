//! Mock implementations of [`Prom`] for testing deferred action recording,
//! plus an on-demand prom-record processor for integration tests.

// Internal organization of the `chapter` module.
mod chapter;
// Internal organization of the `image_task` module.
mod image_task;
// Internal organization of the `invitation` module.
mod invitation;
// Internal organization of the `json` module.
mod json;
// Deferred-record step impls for the mock prom.
mod defer;

// Internal organization of the `tests` module.
mod tests;

use poprako_orchestra::{Nucl as _, OperStep as _, Step};
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use poprako_orchestra_extra::prom::task::Task;
use time::OffsetDateTime;

use poprako_util::i18n::trl;

use self::image_task::ResourceState;
use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::write::page::PageImageRepl;
use crate::model::write::team::TeamAvatarRepl;
use crate::model::write::user::UserAvatarRepl;
use crate::part::image::ImageManager;
use crate::part::prom::Prom;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::comic::{
    GetComicInfoExcluded, MarkComicCoverUploaded,
};
use crate::part::repo::oper::page::{
    GetPageInfoExcluded, MarkPageImageUploaded, SetPageImageUploaded,
};
use crate::part::repo::oper::team::{GetTeamInfoExcluded, UpdateTeam};
use crate::part::repo::oper::user::{GetUserInfoExcluded, UpdateUser};
use crate::part_impl::repo::mock_impl::{Mock, MockContext};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

#[derive(Clone, Copy)]
// Fields and state semantics for the `ImageIdentity` struct.
// Snapshot of the source resource and desired state for a pending image task.
struct ImageIdentity<'a> {
    //
    // Internal state field `kind`.
    // Resource kind (user avatar / team avatar / comic cover / page image) determines the update path.
    kind: image::ResourceKind,
    // Primary key of the resource record.
    resource_id: &'a str,
    // Currently assigned object-storage key.
    object_key: &'a str,
    // Resource version number, used to detect stale writes.
    version: u32,
}

// Fields and state semantics for the `CurrentImageIdentity` struct.
struct CurrentImageIdentity<'a> {
    //
    // Internal state field `version`.
    // Version number of the currently persisted record.
    version: Option<u32>,
    // Object key of the currently persisted record, or None when not yet uploaded.
    object_key: Option<&'a str>,
}

/// A recorded deferred action stored in the mock context during transactional testing.
#[cfg_attr(test, derive(Clone))]
pub struct MockPromRecord {
    //
    // Internal state field `id`.
    /// Server-assigned unique identifier for the prom record.
    id: String,

    /// Serialized JSON of the [`TaskPayload`].
    ///
    /// Call [`payload`](MockPromRecord::payload) to deserialize on-the-fly
    /// for assertions.
    payload_json: String,

    /// Timestamp after which the record is eligible for processing.
    visible_at: OffsetDateTime,
}

impl MockPromRecord {
    /// Returns the prom message id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the deferred visibility time.
    pub fn visible_at(&self) -> OffsetDateTime {
        self.visible_at
    }

    /// Deserializes the stored JSON back into a [`TaskPayload`].
    ///
    pub fn payload(&self) -> TaskPayload {
        serde_json::from_str(&self.payload_json)
            .expect("stored prom payload should deserialize successfully")
    }
}

/// Mock prom implementation used by coordinated tests.
impl Prom<MockContext> for Mock {}

// ── On-demand prom processor for integration tests ─────────────────────────

/// Process all pending prom records in mock state.
///
/// Deserializes each record's stored payload and
/// executes the same handler logic as the production handler against
/// [`Mock`]'s in-memory implementations of all ports.
///
/// Call this after a usecase has enqueued prom records to exercise
/// the full deferred-action chain within an integration test.
pub async fn process_pending(mock: &Mock) -> BaseRest<()> {
    //
    // Internal implementation detail.
    let snapshot = mock.snapshot();

    for record in &snapshot.prom_records {
        //
        // Internal implementation detail.
        let payload = record.payload();

        match payload {
            //
            // Internal implementation detail.
            TaskPayload::Chapter(task) => {
                chapter::process(mock, &task).await?;
            }

            TaskPayload::Image(task) => {
                process_image_task(mock, &task).await?;
            }

            TaskPayload::Invitation(event) => {
                invitation::process(mock, &event).await?;
            }
        }
    }

    accept(())
}

/// Process one image task against the mock state and update related resource
/// records when the referenced object matches expected identity.
// Dispatch the task payload branch by branch and delegate to repository
// mutations that mirror production update handlers.
async fn process_image_task(
    image_pool: &Mock,
    task: &image::ImagePayload,
) -> BaseRest<()> {
    match task {
        //
        // Internal implementation detail.
        image::ImagePayload::CheckUpload {
            resource_kind,
            resource_id,
            object_key,
            version,
        } => match image_pool.object_exists(object_key).await? {
            //
            // Internal implementation detail.
            true => {
                //
                // Internal implementation detail.
                let image_identity = ImageIdentity {
                    kind: *resource_kind,
                    resource_id,
                    object_key,
                    version: *version,
                };

                // Internal implementation detail.
                if image_identity.kind == image::ResourceKind::PageImage {
                    //
                    // Internal implementation detail.
                    let page_info = image_pool
                        .snapshot()
                        .pages
                        .into_iter()
                        .find(|page_info| page_info.id == *resource_id);

                    // SAFETY: stale payload keys are cleaned only by
                    // dedicated Delete tasks.
                    let Some(page_info) = page_info else {
                        return accept(());
                    };

                    if page_info.image_version != Some(image_identity.version) {
                        return accept(());
                    }

                    if page_info.image_key.as_deref()
                        != Some(image_identity.object_key)
                    {
                        return Err(BaseError::Unrecoverable {
                            message: "prom page image version matches but object key differs"
                            .into(),
                        });
                    }
                }

                process_existing_image(image_pool, image_identity, true).await
            }

            false => match resource_kind {
                //
                // Internal implementation detail.
                image::ResourceKind::PageImage => {
                    mark_page_image_unverified(
                        image_pool,
                        resource_id,
                        object_key,
                        *version,
                    )
                    .await
                }

                _ => {
                    //
                    // Internal implementation detail.
                    let image_identity = ImageIdentity {
                        kind: *resource_kind,
                        resource_id,
                        object_key,
                        version: *version,
                    };

                    process_existing_image(image_pool, image_identity, false)
                        .await
                }
            },
        },

        image::ImagePayload::Delete { object_key } => {
            image_pool.delete_object(object_key).await
        }
    }
}

// Internal implementation of `process_existing_image`.
async fn process_existing_image(
    mock: &Mock,
    image_identity: ImageIdentity<'_>,
    image_uploaded: bool,
) -> BaseRest<()> {
    //
    // Internal implementation detail.
    let resource_state = mock
        .coord(async move |context| {
            //
            // Internal implementation detail.
            let resource_state =
                classify_expected_mark(mock, context, image_identity).await?;

            if !matches!(resource_state, ResourceState::Current) {
                return accept(resource_state);
            }

            mark_uploaded(mock, context, image_identity, image_uploaded)
                .await?;

            accept(ResourceState::Current)
        })
        .await?;

    match resource_state {
        //
        // Internal state field ResourceState.
        ResourceState::Current | ResourceState::Stale => accept(()),

        // SAFETY: stale and deleted resource keys belong exclusively to
        // dedicated Delete tasks.
        ResourceState::Missing => accept(()),

        ResourceState::Mismatched => Err(BaseError::Unrecoverable {
            message: "prom image identity does not match current resource"
                .into(),
        }),
    }
}

// Internal implementation of `mark_page_image_unverified`.
async fn mark_page_image_unverified(
    mock: &Mock,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
) -> BaseRest<()> {
    //
    // Internal implementation detail.
    let page_info = mock
        .snapshot()
        .pages
        .into_iter()
        .find(|page_info| page_info.id == resource_id)
        .ok_or_else(|| BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-page-not-found"),
        })?;

    match (
        page_info.image_version == Some(image_version),
        page_info.image_key.as_deref() == Some(object_key),
    ) {
        //
        // Internal implementation detail.
        (false, _) => return accept(()),

        (true, false) => {
            return Err(BaseError::Unrecoverable {
                message:
                    "prom page image version matches but object key differs"
                        .into(),
            });
        }

        (true, true) => {}
    }

    mock.coord(async move |context| {
        //
        // Internal implementation detail.
        GetChapterInfoExcluded {
            id: &page_info.chapter_id,
            incls: &[],
        }
        .step_on(mock, context)
        .await?;

        let locked_page_info = GetPageInfoExcluded { id: resource_id }
            .step_on(mock, context)
            .await?;

        if locked_page_info.image_version != Some(image_version)
            || locked_page_info.image_key.as_deref() != Some(object_key)
        {
            return accept(());
        }

        let repl = PageImageRepl {
            id: resource_id.to_owned(),
            image_version,
            image_key: Some(object_key.to_owned()),
            is_image_uploaded: false,
        };

        SetPageImageUploaded { repl: &repl }
            .step_on(mock, context)
            .await?;

        accept(())
    })
    .await
    .map_err(Into::into)
}

// Internal implementation of `classify_expected_mark`.
async fn classify_expected_mark(
    mock: &Mock,
    context: &mut MockContext,
    image_identity: ImageIdentity<'_>,
) -> BaseRest<ResourceState> {
    match image_identity.kind {
        //
        // Internal implementation detail.
        image::ResourceKind::UserAvatar => {
            match (GetUserInfoExcluded::Id {
                id: image_identity.resource_id,
            })
            .step_on(mock, context)
            .await
            {
                // Internal implementation detail.
                Ok(user_info) => {
                    //
                    // Internal implementation detail.
                    let current_identity = CurrentImageIdentity {
                        version: user_info.avatar_version,
                        object_key: user_info.avatar_key.as_deref(),
                    };

                    classify_current_identity(current_identity, image_identity)
                }

                Err(BaseError::Expected { .. }) => {
                    accept(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }

        image::ResourceKind::TeamAvatar => {
            match (GetTeamInfoExcluded::Id {
                id: image_identity.resource_id,
            })
            .step_on(mock, context)
            .await
            {
                // Internal implementation detail.
                Ok(team_info) => {
                    //
                    // Internal implementation detail.
                    let current_identity = CurrentImageIdentity {
                        version: team_info.avatar_version,
                        object_key: team_info.avatar_key.as_deref(),
                    };

                    classify_current_identity(current_identity, image_identity)
                }

                Err(BaseError::Expected { .. }) => {
                    accept(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }

        image::ResourceKind::ComicCover => {
            match (GetComicInfoExcluded {
                id: image_identity.resource_id,
                incls: &[],
            })
            .step_on(mock, context)
            .await
            {
                Ok(comic_info) => {
                    //
                    // Internal implementation detail.
                    let current_identity = CurrentImageIdentity {
                        version: comic_info.cover_version,
                        object_key: comic_info.cover_key.as_deref(),
                    };

                    classify_current_identity(current_identity, image_identity)
                }

                Err(BaseError::Expected { .. }) => {
                    accept(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }

        image::ResourceKind::PageImage => {
            match (GetPageInfoExcluded {
                id: image_identity.resource_id,
            })
            .step_on(mock, context)
            .await
            {
                Ok(page_info) => {
                    //
                    // Internal implementation detail.
                    let current_identity = CurrentImageIdentity {
                        version: page_info.image_version,
                        object_key: page_info.image_key.as_deref(),
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

// Internal implementation of `mark_uploaded`.
async fn mark_uploaded(
    mock: &Mock,
    context: &mut MockContext,
    image_identity: ImageIdentity<'_>,
    image_uploaded: bool,
) -> BaseRest<()> {
    match image_identity.kind {
        //
        // Internal implementation detail.
        image::ResourceKind::UserAvatar => {
            let repl = UserAvatarRepl {
                id: image_identity.resource_id.to_owned(),
                avatar_version: image_identity.version,
                avatar_key: Some(image_identity.object_key.to_owned()),
                is_avatar_uploaded: image_uploaded,
            };

            UpdateUser::MarkAvatarUploaded { repl: &repl }
                .step_on(mock, context)
                .await
        }

        image::ResourceKind::TeamAvatar => {
            let repl = TeamAvatarRepl {
                id: image_identity.resource_id.to_owned(),
                avatar_version: image_identity.version,
                avatar_key: Some(image_identity.object_key.to_owned()),
                is_avatar_uploaded: image_uploaded,
            };

            UpdateTeam::MarkAvatarUploaded { repl: &repl }
                .step_on(mock, context)
                .await
        }

        image::ResourceKind::ComicCover => {
            MarkComicCoverUploaded {
                id: image_identity.resource_id,
                cover_version: image_identity.version,
                cover_key: Some(image_identity.object_key),
                cover_uploaded: image_uploaded,
            }
            .step_on(mock, context)
            .await
        }

        image::ResourceKind::PageImage => {
            //
            // Internal implementation detail.
            let page_info = context
                .state
                .pages
                .iter()
                .find(|page_info| page_info.id == image_identity.resource_id)
                .cloned()
                .ok_or_else(|| BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: trl("error-page-not-found"),
                })?;

            GetChapterInfoExcluded {
                id: &page_info.chapter_id,
                incls: &[],
            }
            .step_on(mock, context)
            .await?;

            let repl = PageImageRepl {
                id: image_identity.resource_id.to_owned(),
                image_version: image_identity.version,
                image_key: Some(image_identity.object_key.to_owned()),
                is_image_uploaded: true,
            };

            MarkPageImageUploaded { repl: &repl }
                .step_on(mock, context)
                .await?;

            accept(())
        }
    }
}

// Internal implementation of `classify_current_identity`.
fn classify_current_identity(
    current_identity: CurrentImageIdentity<'_>,
    image_identity: ImageIdentity<'_>,
) -> BaseRest<ResourceState> {
    match (
        current_identity.version == Some(image_identity.version),
        current_identity.object_key == Some(image_identity.object_key),
    ) {
        //
        // Internal implementation detail.
        (false, _) => accept(ResourceState::Stale),

        (true, false) => accept(ResourceState::Mismatched),

        (true, true) => accept(ResourceState::Current),
    }
}
