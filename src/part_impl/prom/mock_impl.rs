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
use crate::part::prom::payload::{Payload, image};
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
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};

mod chapter;
mod image_task;
mod invitation;
mod tests;

/// A recorded deferred action stored in the mock context during transactional testing.
#[cfg_attr(test, derive(Clone))]
pub struct MockPromRecord {
    /// Server-assigned unique identifier for the prom record.
    id: String,

    /// Serialized JSON of the [`Payload`].
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

    /// Deserializes the stored JSON back into a [`Payload`].
    ///
    pub fn payload(&self) -> Payload {
        serde_json::from_str(&self.payload_json)
            .expect("stored prom payload should deserialize successfully")
    }
}

/// Mock prom implementation used by coordinated tests.
impl Prom<MockContext> for Mock {}

/// Defers one record in the coordinated mock state.
impl<'a> Step<Defer<'a, String, Payload, ()>, MockContext> for Mock {
    type Error = BaseError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &Defer<'a, String, Payload, ()>,
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

impl<'t, 'a> Step<DeferBatch<'t, 'a, String, Payload, ()>, MockContext>
    for Mock
{
    type Error = BaseError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeferBatch<'t, 'a, String, Payload, ()>,
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
            Payload::AdvanceRawProvide(task) => {
                chapter::process_advance_raw_provide(mock, &task).await?;
            }

            Payload::Image(task) => {
                process_image_task(mock, &task).await?;
            }

            Payload::PurgeExpiredInvitation(event) => {
                invitation::process(mock, &event).await?;
            }
        }
    }

    accept(())
}

/// Process a single image task against the mock's in-memory image pool.
async fn process_image_task(
    image_pool: &Mock,
    task: &image::Payload,
) -> BaseResult<()> {
    match task {
        //
        image::Payload::CheckUpload {
            resource_kind,
            resource_id,
            object_key,
            version,
            image_hash,
            image_ext,
        } => match image_pool.head_object(object_key).await? {
            //
            Some(object_info) => {
                //
                if *resource_kind == image::ResourceKind::PageImage {
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

                    if page_info.image_version != *version {
                        return accept(());
                    }

                    if page_info.image_key.as_deref() != Some(object_key) {
                        return Err(BaseError::Unrecoverable {
                            message: "prom page image version matches but object key differs"
                                .into(),
                        });
                    }

                    if page_info.image_hash != *image_hash
                        || page_info.image_ext != *image_ext
                    {
                        return Err(BaseError::Unrecoverable {
                            message: "prom page image payload identity differs from current resource"
                                .into(),
                        });
                    }

                    if *image_hash != object_info.checksum_sha256 {
                        //
                        mark_page_image_unverified(
                            image_pool,
                            resource_id,
                            object_key,
                            *version,
                        )
                        .await?;

                        return image_pool.delete_object(object_key).await;
                    }
                }

                process_existing_image(
                    image_pool,
                    *resource_kind,
                    resource_id,
                    object_key,
                    *version,
                    image_hash,
                    *image_ext,
                    object_info.checksum_sha256 == *image_hash,
                    object_info.checksum_sha256 != *image_hash,
                )
                .await
            }

            None => match resource_kind {
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
                    process_existing_image(
                        image_pool,
                        *resource_kind,
                        resource_id,
                        object_key,
                        *version,
                        image_hash,
                        *image_ext,
                        false,
                        false,
                    )
                    .await
                }
            },
        },

        image::Payload::Delete { object_key } => {
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
    kind: image::ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
    image_hash: &crate::value::image::ImageHash,
    image_ext: crate::value::image::ImageExt,
    image_uploaded: bool,
    delete_mismatch: bool,
) -> BaseResult<()> {
    //
    let resource_state = mock
        .coord(async move |context| {
            //
            let resource_state = classify_expected_mark(
                mock,
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

            mark_uploaded(
                mock,
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

    match resource_state {
        //
        ResourceState::Current if delete_mismatch => {
            mock.delete_object(object_key).await
        }

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
    kind: image::ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
    image_uploaded: bool,
) -> BaseResult<()> {
    match kind {
        //
        image::ResourceKind::UserAvatar => {
            mock.step(
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

        image::ResourceKind::TeamAvatar => {
            mock.step(
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

        image::ResourceKind::ComicCover => {
            mock.step(
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

        image::ResourceKind::PageImage => {
            //
            let page_info = context
                .state
                .pages
                .iter()
                .find(|page_info| page_info.id == resource_id)
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
                    id: resource_id,
                    image_version,
                    image_key: Some(object_key),
                },
            )
            .await?;

            accept(())
        }
    }
}

fn classify_current_identity(
    current_version: u32,
    current_object_key: Option<&str>,
    image_version: u32,
    object_key: &str,
    current_hash: &crate::value::image::ImageHash,
    current_ext: crate::value::image::ImageExt,
    image_hash: &crate::value::image::ImageHash,
    image_ext: crate::value::image::ImageExt,
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

async fn classify_expected_mark(
    mock: &Mock,
    context: &mut MockContext,
    kind: image::ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
    image_hash: &crate::value::image::ImageHash,
    image_ext: crate::value::image::ImageExt,
) -> BaseResult<ResourceState> {
    match kind {
        //
        image::ResourceKind::UserAvatar => {
            match mock
                .step(context, &GetUserInfoExcluded::Id { id: resource_id })
                .await
            {
                //
                Ok(user_info) => classify_current_identity(
                    user_info.avatar_version,
                    user_info.avatar_key.as_deref(),
                    image_version,
                    object_key,
                    &user_info.avatar_hash,
                    user_info.avatar_ext,
                    image_hash,
                    image_ext,
                ),

                Err(BaseError::Expected { .. }) => {
                    accept(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }

        image::ResourceKind::TeamAvatar => {
            match mock
                .step(context, &GetTeamInfoExcluded::Id { id: resource_id })
                .await
            {
                //
                Ok(team_info) => classify_current_identity(
                    team_info.avatar_version,
                    team_info.avatar_key.as_deref(),
                    image_version,
                    object_key,
                    &team_info.avatar_hash,
                    team_info.avatar_ext,
                    image_hash,
                    image_ext,
                ),

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
                    &comic_info.cover_hash,
                    comic_info.cover_ext,
                    image_hash,
                    image_ext,
                ),

                Err(BaseError::Expected { .. }) => {
                    accept(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }

        image::ResourceKind::PageImage => {
            match mock
                .step(context, &GetPageInfoExcluded { id: resource_id })
                .await
            {
                Ok(page_info) => classify_current_identity(
                    page_info.image_version,
                    page_info.image_key.as_deref(),
                    image_version,
                    object_key,
                    &page_info.image_hash,
                    page_info.image_ext,
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
