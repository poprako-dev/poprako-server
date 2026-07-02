//! Mock implementations of [PromTransactional] for testing deferred action recording.

use async_trait::async_trait;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::part::prom::{Append, Payload, Prom};
use crate::part_impl::repo_mock::{Mock, MockContext, MockTransactional};
use crate::result::RegularError;

/// A recorded deferred action stored in the mock context during transactional testing.
#[cfg_attr(test, derive(Clone))]
pub struct MockPromRecord {
    pub id: String,
    pub topic: String,

    /// Serialized JSON of the [`Payload`].
    ///
    /// Call [`payload`](MockPromRecord::payload) to deserialize on-the-fly
    /// for assertions.
    payload_json: String,

    pub visible_at: OffsetDateTime,
}

impl MockPromRecord {
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
            serde_json::to_string(&step.payload).map_err(|e| RegularError::Unrecoverable {
                message: format!("failed to serialize prom payload: {}", e),
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
            serde_json::to_string(&step.payload).map_err(|e| RegularError::Unrecoverable {
                message: format!("failed to serialize prom payload: {}", e),
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

use poprako_transactional::drive::Drive;

use crate::part::prom::PromStep;
use crate::part::prom::task::ImageTask;
use crate::result::accept;

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
            accept(())
        })
        .await
        .is_ok()
    );

    let snapshot = mock.snapshot();
    assert_eq!(snapshot.prom_records.len(), 1);
    assert_eq!(snapshot.prom_records[0].id, "prom-1");
}
