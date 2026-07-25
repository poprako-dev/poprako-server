//! Mock implementations of [`Prom`] for testing deferred action recording,
//! plus an on-demand prom-record processor for integration tests.

use poprako_orchestra::{Nucl as _, Step};
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use poprako_orchestra_extra::prom::task::Task;
use time::OffsetDateTime;

use poprako_util::i18n::trl;

use self::image_task::ResourceState;
use crate::model::user::{UserCredential, UserInfo};
use crate::part::image::ImageManager;
use crate::part::prom::Prom;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::comic::{GetComicInfoExcluded, MarkComicCoverUploaded};
use crate::part::repo::oper::page::{GetPageInfoExcluded, MarkPageImageUploaded, SetPageImageUploaded};
use crate::part::repo::oper::team::{GetTeamInfoExcluded, UpdateTeam};
use crate::part::repo::oper::user::{GetUserInfoExcluded, UpdateUser};
use crate::part_impl::repo::mock_impl::{Mock, MockContext};
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};

mod chapter;
mod image_task;
mod invitation;
mod tests;

#[derive(Clone, Copy)]
struct ImageIdentity<'a> {
    //
    kind: image::ResourceKind,
    resource_id: &'a str,
    object_key: &'a str,
    version: u32,
}

struct CurrentImageIdentity<'a> {
    //
    version: u32,
    object_key: Option<&'a str>,
}

/// A recorded deferred action stored in the mock context during transactional testing.
#[cfg_attr(test, derive(Clone))]
pub struct MockPromRecord {
    //
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

/// Defers one record in the coordinated mock state.
impl<'a> Step<Defer<'a, String, TaskPayload, ()>, MockContext> for Mock {
    type Error = BaseError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &Defer<'a, String, TaskPayload, ()>,
    ) -> Result<(), Self::Error> {
        //
        let payload_json =
            serde_json::to_string(oper.task.payload).map_err(|error| {
                BaseError::Unrecoverable {
                    message: format!(
                        "failed to serialize prom payload: {}",
                        error
                    ),
                }
            })?;

        context.state.prom_records.push(MockPromRecord {
            id: oper.task.id.to_string(),
            payload_json,
            visible_at: OffsetDateTime::now_utc()
                + oper.task.delay.unwrap_or_default(),
        });

        accept(())
    }
}

impl<'t, 'a> Step<DeferBatch<'t, 'a, String, TaskPayload, ()>, MockContext>
    for Mock
{
    type Error = BaseError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeferBatch<'t, 'a, String, TaskPayload, ()>,
    ) -> Result<(), Self::Error> {
        //
        for task in oper.tasks {
            self.step(
                context,
                &Defer::new(Task {
                    id: task.id,
                    payload: task.payload,
                    delay: task.delay,
                }),
            )
            .await?;
        }

        accept(())
    }
}

// ── On-demand prom processor for integration tests ─────────────────────────

// ── On-demand prom processor for integration tests ─────────────────────────

/// Process all pending prom records in mock state.
///
/// Deserializes each record's stored payload and
/// executes the same handler logic as the production handler against
/// [`Mock`]'s in-memory implementations of all ports.
///
/// Call this after a usecase has enqueued prom records to exercise
/// the full deferred-action chain within an integration test.
pub async fn process_pending(mock: &Mock) -> BaseResult<()> {
    //
    let snapshot = mock.snapshot();

    for record in &snapshot.prom_records {
        //
        let payload = record.payload();

        match payload {
            //
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

/// Process a single image task against the mock's in-memory image pool.
async fn process_image_task(
    image_pool: &Mock,
    task: &image::ImagePayload,
) -> BaseResult<()> {
    match task {
        //
        image::ImagePayload::CheckUpload {
            resource_kind,
            resource_id,
            object_key,
            version,
        } => match image_pool.object_exists(object_key).await? {
            //
            true => {
                //
                let image_identity = ImageIdentity {
                    kind: *resource_kind,
                    resource_id,
                    object_key,
                    version: *version,
                };

                //
                if image_identity.kind == image::ResourceKind::PageImage {
                    //
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

                    if page_info.image_version != image_identity.version {
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

async fn mark_page_image_unverified(
    mock: &Mock,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
) -> BaseResult<()> {
    //
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
        page_info.image_version == image_version,
        page_info.image_key.as_deref() == Some(object_key),
    ) {
        //
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
        mock.step(
            context,
            &GetChapterInfoExcluded {
                id: &page_info.chapter_id,
                incls: &[],
            },
        )
        .await?;

        let locked_page_info = mock
            .step(context, &GetPageInfoExcluded { id: resource_id })
            .await?;

        if locked_page_info.image_version != image_version
            || locked_page_info.image_key.as_deref() != Some(object_key)
        {
            return accept(());
        }

        mock.step(
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
    .map_err(Into::into)
}

async fn process_existing_image(
    mock: &Mock,
    image_identity: ImageIdentity<'_>,
    image_uploaded: bool,
) -> BaseResult<()> {
    //
    let resource_state = mock
        .coord(async move |context| {
            //
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

async fn mark_uploaded(
    mock: &Mock,
    context: &mut MockContext,
    image_identity: ImageIdentity<'_>,
    image_uploaded: bool,
) -> BaseResult<()> {
    match image_identity.kind {
        //
        image::ResourceKind::UserAvatar => {
            mock.step(
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

        image::ResourceKind::TeamAvatar => {
            mock.step(
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

        image::ResourceKind::ComicCover => {
            mock.step(
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

        image::ResourceKind::PageImage => {
            //
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

            mock.step(
                context,
                &GetChapterInfoExcluded {
                    id: &page_info.chapter_id,
                    incls: &[],
                },
            )
            .await?;

            mock.step(
                context,
                &MarkPageImageUploaded {
                    id: image_identity.resource_id,
                    image_version: image_identity.version,
                    image_key: Some(image_identity.object_key),
                },
            )
            .await?;

            accept(())
        }
    }
}

fn classify_current_identity(
    current_identity: CurrentImageIdentity<'_>,
    image_identity: ImageIdentity<'_>,
) -> BaseResult<ResourceState> {
    match (
        current_identity.version == image_identity.version,
        current_identity.object_key == Some(image_identity.object_key),
    ) {
        //
        (false, _) => accept(ResourceState::Stale),

        (true, false) => accept(ResourceState::Mismatched),

        (true, true) => accept(ResourceState::Current),
    }
}

async fn classify_expected_mark(
    mock: &Mock,
    context: &mut MockContext,
    image_identity: ImageIdentity<'_>,
) -> BaseResult<ResourceState> {
    match image_identity.kind {
        //
        image::ResourceKind::UserAvatar => {
            match mock
                .step(
                    context,
                    &GetUserInfoExcluded::Id {
                        id: image_identity.resource_id,
                    },
                )
                .await
            {
                //
                Ok(user_info) => {
                    //
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
            match mock
                .step(
                    context,
                    &GetTeamInfoExcluded::Id {
                        id: image_identity.resource_id,
                    },
                )
                .await
            {
                //
                Ok(team_info) => {
                    //
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
            match mock
                .step(
                    context,
                    &GetComicInfoExcluded {
                        id: image_identity.resource_id,
                        incls: &[],
                    },
                )
                .await
            {
                Ok(comic_info) => {
                    //
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
            match mock
                .step(
                    context,
                    &GetPageInfoExcluded {
                        id: image_identity.resource_id,
                    },
                )
                .await
            {
                Ok(page_info) => {
                    //
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
