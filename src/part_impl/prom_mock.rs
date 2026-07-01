//! Mock implementations of [PromTransactional] for testing deferred action recording.

use async_trait::async_trait;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::part::prom::{Append, Payload, PromTransactional};
use crate::part_impl::repo_mock::{Mock, MockContext, MockTransactional};
use crate::result::RegularError;

/// A recorded deferred action stored in the mock context during transactional testing.
#[cfg_attr(test, derive(Clone))]
pub struct MockPromRecord {
    pub id: String,
    pub topic: String,
    pub payload: Payload,
    pub visible_at: OffsetDateTime,
}

/// Empty mock implementation of [PromTransactional] — actual advancement is handled by [Advance].
impl PromTransactional<MockContext> for MockTransactional {}

/// Empty mock implementation of [PromTransactional] on [`Mock`] so tests can pass
/// `&mock` directly as both repo and prom argument.
impl PromTransactional<MockContext> for Mock {}

/// Appends a [MockPromRecord] to the mock context state when a prom append is advanced.
#[async_trait]
impl<'a> Advance<Append<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Append<'a>,
    ) -> Result<(), Self::Error> {
        context.state.prom_records.push(MockPromRecord {
            id: step.id.to_string(),
            topic: step.topic.to_string(),
            payload: step.payload.clone(),
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
        context.state.prom_records.push(MockPromRecord {
            id: step.id.to_string(),
            topic: step.topic.to_string(),
            payload: step.payload.clone(),
            visible_at: *step.visible_at,
        });
        Ok(())
    }
}

// append_records_payload(PromTransactional::advance)(positive): prom append should store the record in transaction state.

use poprako_transactional::drive::Drive;

use crate::part::prom::PromStep;
use crate::part::prom::intention::ImageIntention;
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
                    Payload::Image(ImageIntention::Delete {
                        object_key: "key".into(),
                    }),
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
