use async_trait::async_trait;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::part::prom::{Append, Payload, Prom, PromTransactional};
use crate::part_impl::repo_mock::{Mock, MockContext, MockTransactional};
use crate::result::RootError;

#[cfg_attr(test, derive(Clone))]
pub struct MockPromRecord {
    pub id: String,
    pub topic: String,
    pub payload: Payload,
    pub visible_at: OffsetDateTime,
}

impl Prom<MockContext> for Mock {}

impl PromTransactional<MockContext> for MockTransactional {}

#[async_trait]
impl<'a> Advance<Append<'a>, MockContext> for MockTransactional {
    type Error = RootError;

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

mod tests {
    // append_records_payload(PromTransactional::advance)(positive): prom append should store the record in transaction state.

    use super::*;

    use poprako_transactional::drive::Drive;

    use crate::part::prom::intention::ImageIntention;
    use crate::part::prom::{Payload, PromStep};
    use crate::result::accept;

    #[tokio::test]
    async fn append_records_payload() {
        let mock = Mock::new();
        let visible_at = OffsetDateTime::now_utc();

        let result = Drive::with_context(&mock, async move |context| {
            let txn = MockTransactional;
            Advance::advance(
                &txn,
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
        .await;
        assert!(result.is_ok());

        let snapshot = mock.snapshot();
        assert_eq!(snapshot.prom_records.len(), 1);
        assert_eq!(snapshot.prom_records[0].id, "prom-1");
    }
}
