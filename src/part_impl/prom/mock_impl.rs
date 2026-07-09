//! Mock implementations of [PromTransactional] for testing deferred action recording,
//! plus an on-demand prom-record processor for integration tests.

use async_trait::async_trait;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::complex::comic::ComicComplex;
use crate::model::user::{UserCredential, UserInfo};
use crate::part::image::ImagePool;
use crate::part::prom::task::{
    COMIC_ARCHIVE_TOPIC, ComicTask, IMAGE_TOPIC, ImageKind, ImageTask,
};
use crate::part::prom::{Append, Payload, Prom};
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::step::page::PageStep;
use crate::part::repo::step::team::TeamStep;
use crate::part::repo::step::user::UserStep;
use crate::part_impl::repo::mock_impl::{Mock, MockContext, MockTransactional};
use crate::result::{RegularError, RegularResult};

/// A recorded deferred action stored in the mock context during transactional testing.
#[cfg_attr(test, derive(Clone))]
pub struct MockPromRecord {
    id: String,
    topic: String,

    /// Serialized JSON of the [`Payload`].
    ///
    /// Call [`payload`](MockPromRecord::payload) to deserialize on-the-fly
    /// for assertions.
    payload_json: String,

    visible_at: OffsetDateTime,
}

impl MockPromRecord {
    /// Returns the prom message id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the prom topic.
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// Returns the deferred visibility time.
    pub fn visible_at(&self) -> OffsetDateTime {
        self.visible_at
    }

    /// Deserializes the stored JSON back into a [`Payload`].
    ///
    /// The returned `Payload` borrows from `self`, so it's valid for
    /// the duration of the borrow.
    pub fn payload(&self) -> Payload<'_> {
        serde_json::from_str(&self.payload_json)
            .expect("stored prom payload should deserialize successfully")
    }
}

/// Empty mock implementation of [PromTransactional] — actual advancement is handled by [Advance].
impl Prom<MockContext> for MockTransactional {}

/// Empty mock implementation of [PromTransactional] on [`Mock`] so tests can pass
/// `&mock` directly as both repo and prom argument.
impl Prom<MockContext> for Mock {}

/// Appends a [MockPromRecord] to the mock context state when a prom append is advanced.
#[async_trait]
impl<'a> Advance<Append<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Append<'a>,
    ) -> Result<(), Self::Error> {
        let payload_json =
            serde_json::to_string(&step.payload).map_err(|e| {
                RegularError::Unrecoverable {
                    message: format!("failed to serialize prom payload: {}", e),
                }
            })?;

        context.state.prom_records.push(MockPromRecord {
            id: step.id.to_string(),
            topic: step.topic.to_string(),
            payload_json,
            visible_at: *step.visible_at,
        });
        Ok(())
    }
}

/// Prom append on [`Mock`] delegates to the same mock state `prom_records`.
#[async_trait]
impl<'a> Advance<Append<'a>, MockContext> for Mock {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Append<'a>,
    ) -> Result<(), Self::Error> {
        let payload_json =
            serde_json::to_string(&step.payload).map_err(|e| {
                RegularError::Unrecoverable {
                    message: format!("failed to serialize prom payload: {}", e),
                }
            })?;

        context.state.prom_records.push(MockPromRecord {
            id: step.id.to_string(),
            topic: step.topic.to_string(),
            payload_json,
            visible_at: *step.visible_at,
        });
        Ok(())
    }
}

// append_records_payload(PromTransactional::advance)(positive): prom append should store the record in transaction state.
// process_pending_marks_uploaded_image(process_pending)(positive): check-upload should mark matching uploaded image state.
// process_pending_keeps_stale_image_for_ordered_delete(process_pending)(positive): stale check-upload should leave cleanup to ordered delete messages.
// process_pending_deletes_missing_resource_image(process_pending)(positive): check-upload should delete an object when the owning resource disappeared.

use crate::part::prom::PromStep;

fn user_info(id: &str, avatar_key: &str, avatar_version: i64) -> UserInfo {
    let now = OffsetDateTime::now_utc();

    UserInfo {
        id: id.to_string(),
        qid: format!("qid-{}", id),
        nickname: format!("nick-{}", id),
        avatar_key: Some(avatar_key.to_string()),
        avatar_uploaded: false,
        avatar_version,
        is_sadmin: false,
        last_active_at: now,
        created_at: now,
        updated_at: now,
    }
}

fn user_credential(id: &str) -> UserCredential {
    UserCredential {
        user_id: id.to_string(),
        password_hash: format!("hash-{}", id),
    }
}

#[tokio::test]
async fn append_records_payload() {
    let mock = Mock::new();
    let visible_at = OffsetDateTime::now_utc();

    assert!(
        Drive::with_context(&mock, async move |context| {
            let transactional = MockTransactional;
            Advance::advance(
                &transactional,
                context,
                &PromStep::append(
                    "prom-1",
                    "image",
                    Payload::Image(ImageTask::Delete { object_key: "key" }),
                    &visible_at,
                ),
            )
            .await?;
            Ok::<(), RegularError>(())
        })
        .await
        .is_ok()
    );

    let snapshot = mock.snapshot();
    assert_eq!(snapshot.prom_records.len(), 1);
    assert_eq!(snapshot.prom_records[0].id(), "prom-1");
    assert_eq!(snapshot.prom_records[0].topic(), "image");
    assert_eq!(snapshot.prom_records[0].visible_at(), visible_at);
}

#[tokio::test]
async fn process_pending_marks_uploaded_image() {
    let mock = Mock::new();
    let visible_at = OffsetDateTime::now_utc();

    mock.seed_user(
        user_info("user-1", "avatar.png", 1),
        user_credential("user-1"),
    );

    Drive::with_context(&mock, async move |context| {
        let transactional = MockTransactional;

        Advance::advance(
            &transactional,
            context,
            &PromStep::append(
                "prom-1",
                IMAGE_TOPIC,
                Payload::Image(ImageTask::CheckUploaded {
                    kind: ImageKind::UserAvatar,
                    resource_id: "user-1",
                    object_key: "avatar.png",
                    image_version: 1,
                }),
                &visible_at,
            ),
        )
        .await?;

        Ok::<(), RegularError>(())
    })
    .await
    .ok()
    .unwrap();

    process_pending(&mock).await.ok().unwrap();

    assert!(mock.snapshot().users[0].avatar_uploaded);
}

#[tokio::test]
async fn process_pending_keeps_stale_image_for_ordered_delete() {
    let mock = Mock::new();
    let visible_at = OffsetDateTime::now_utc();

    mock.seed_user(
        user_info("user-1", "avatar-v2.png", 2),
        user_credential("user-1"),
    );

    Drive::with_context(&mock, async move |context| {
        let transactional = MockTransactional;

        Advance::advance(
            &transactional,
            context,
            &PromStep::append(
                "prom-1",
                IMAGE_TOPIC,
                Payload::Image(ImageTask::CheckUploaded {
                    kind: ImageKind::UserAvatar,
                    resource_id: "user-1",
                    object_key: "avatar-v1.png",
                    image_version: 1,
                }),
                &visible_at,
            ),
        )
        .await?;

        Ok::<(), RegularError>(())
    })
    .await
    .ok()
    .unwrap();

    process_pending(&mock).await.ok().unwrap();

    let snapshot = mock.snapshot();

    assert!(!snapshot.users[0].avatar_uploaded);
    assert!(snapshot.deleted_image_keys.is_empty());
}

#[tokio::test]
async fn process_pending_deletes_missing_resource_image() {
    let mock = Mock::new();
    let visible_at = OffsetDateTime::now_utc();

    Drive::with_context(&mock, async move |context| {
        let transactional = MockTransactional;

        Advance::advance(
            &transactional,
            context,
            &PromStep::append(
                "prom-1",
                IMAGE_TOPIC,
                Payload::Image(ImageTask::CheckUploaded {
                    kind: ImageKind::UserAvatar,
                    resource_id: "missing-user",
                    object_key: "orphan-avatar.png",
                    image_version: 1,
                }),
                &visible_at,
            ),
        )
        .await?;

        Ok::<(), RegularError>(())
    })
    .await
    .ok()
    .unwrap();

    process_pending(&mock).await.ok().unwrap();

    assert_eq!(
        mock.snapshot().deleted_image_keys,
        vec!["orphan-avatar.png".to_string()]
    );
}

// ── On-demand prom processor for integration tests ─────────────────────────

/// Process all pending prom records in mock state.
///
/// Deserializes each record's stored payload, routes by topic, and
/// executes the same handler logic as the production handler against
/// [`Mock`]'s in-memory implementations of all ports.
///
/// Call this after a usecase has enqueued prom records to exercise
/// the full deferred-action chain within an integration test.
pub async fn process_pending(mock: &Mock) -> RegularResult<()> {
    let snapshot = mock.snapshot();

    for record in &snapshot.prom_records {
        let payload = record.payload();

        match record.topic() {
            IMAGE_TOPIC => {
                if let Payload::Image(ref task) = payload {
                    process_image_task(mock, task).await?;
                }
            }
            COMIC_ARCHIVE_TOPIC => {
                if let Payload::Comic(ref task) = payload {
                    process_comic_task(mock, task).await?;
                }
            }
            unknown => {
                return Err(RegularError::Unrecoverable {
                    message: format!("unknown prom topic in mock: {}", unknown),
                });
            }
        }
    }

    Ok(())
}

/// Process a single image task against the mock's in-memory image pool.
async fn process_image_task(
    mock: &Mock,
    task: &ImageTask<'_>,
) -> RegularResult<()> {
    match task {
        ImageTask::CheckUploaded {
            kind,
            resource_id,
            object_key,
            image_version,
        } => match ImagePool::head_object(mock, object_key).await? {
            true => {
                process_existing_image(
                    mock,
                    *kind,
                    resource_id,
                    object_key,
                    *image_version,
                )
                .await
            }
            false => Ok(()),
        },
        ImageTask::Delete { object_key } => {
            ImagePool::delete_object(mock, object_key).await
        }
    }
}

async fn process_existing_image(
    mock: &Mock,
    kind: ImageKind,
    resource_id: &str,
    object_key: &str,
    image_version: i64,
) -> RegularResult<()> {
    let mark_result = Drive::with_context(mock, async move |context| {
        let transactional = MockTransactional;

        mark_uploaded(&transactional, context, kind, resource_id, image_version)
            .await
    })
    .await
    .map_err(|e| e.into());

    match mark_result {
        Ok(()) => Ok(()),
        Err(RegularError::Expected { .. }) => {
            match mock_resource_exists(mock, kind, resource_id) {
                true => Ok(()),
                false => ImagePool::delete_object(mock, object_key).await,
            }
        }
        Err(e) => Err(e),
    }
}

async fn mark_uploaded(
    transactional: &MockTransactional,
    context: &mut MockContext,
    kind: ImageKind,
    resource_id: &str,
    image_version: i64,
) -> RegularResult<()> {
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

fn mock_resource_exists(
    mock: &Mock,
    kind: ImageKind,
    resource_id: &str,
) -> bool {
    let snapshot = mock.snapshot();

    match kind {
        ImageKind::UserAvatar => snapshot
            .users
            .iter()
            .any(|user_info| user_info.id == resource_id),
        ImageKind::TeamAvatar => snapshot
            .teams
            .iter()
            .any(|team_info| team_info.id == resource_id),
        ImageKind::ComicCover => snapshot
            .comics
            .iter()
            .any(|comic_info| comic_info.id == resource_id),
        ImageKind::PageImage => snapshot
            .pages
            .iter()
            .any(|page_info| page_info.id == resource_id),
    }
}

/// Process a single comic archive task against the mock's in-memory repository.
async fn process_comic_task(
    mock: &Mock,
    task: &ComicTask<'_>,
) -> RegularResult<()> {
    match task {
        ComicTask::Archive { comic_id } => {
            let comic_id = comic_id.to_string();

            Drive::with_context(mock, async move |context| {
                let transactional = MockTransactional;

                ComicComplex::delete_cascade(
                    &transactional,
                    mock,
                    context,
                    &comic_id,
                )
                .await
            })
            .await
            .map_err(|e| e.into())
        }
    }
}
