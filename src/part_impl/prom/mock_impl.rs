//! Mock implementations of [PromTransactional] for testing deferred action recording,
//! plus an on-demand prom-record processor for integration tests.

use async_trait::async_trait;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::complex::comic::ComicComplex;
use crate::part::image::ImagePool;
use crate::part::prom::task::{
    COMIC_ARCHIVE_TOPIC, ComicTask, IMAGE_TOPIC, ImageTask,
};
use crate::part::prom::{Append, Payload, Prom};
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

use crate::part::prom::PromStep;

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
            Ok(())
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

async fn process_image_task(
    mock: &Mock,
    task: &ImageTask<'_>,
) -> RegularResult<()> {
    match task {
        ImageTask::CheckUploaded { object_key, .. } => {
            let exists = ImagePool::head_object(mock, object_key).await?;
            let _ = exists;
            Ok(())
        }
        ImageTask::Delete { object_key } => {
            ImagePool::delete_object(mock, object_key).await
        }
    }
}

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
