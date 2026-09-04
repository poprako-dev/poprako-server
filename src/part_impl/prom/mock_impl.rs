//! Mock deferred-task recording and on-demand processing.

mod defer;
mod json;

#[cfg(test)]
mod tests;

use time::OffsetDateTime;

use crate::part::prom::payload::TaskPayload;
use crate::part_impl::prom::dispatch;
use crate::part_impl::prom::task_flow::TaskFlow;
use crate::part_impl::repo::mock_impl::Mock;
use crate::part_impl::repo::mock_impl::MockContext;
use crate::result::{BaseError, BaseRest, accept};

/// One deferred action recorded by the mock transaction context.
#[cfg_attr(test, derive(Clone))]
pub struct MockPromRecord {
    pub(super) id: String,
    pub(super) payload_json: String,
    pub(super) visible_at: OffsetDateTime,
}

impl MockPromRecord {
    /// Returns the durable message identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the first processing time.
    pub fn visible_at(&self) -> OffsetDateTime {
        self.visible_at
    }

    /// Decodes the stored payload for assertions and processing.
    pub fn payload(&self) -> TaskPayload {
        serde_json::from_str(&self.payload_json)
            .expect("stored prom payload should deserialize successfully")
    }
}

/// Processes every recorded non-object deferred action.
pub async fn process_pending(mock: &Mock) -> BaseRest<()> {
    let snapshot = mock.snapshot();

    for record in &snapshot.prom_records {
        let flow = dispatch::dispatch::<MockContext, _, _, _, _>(
            (mock, mock, mock, mock),
            record.payload(),
        )
        .await;

        match flow {
            TaskFlow::Complete | TaskFlow::Wait { .. } => {}

            TaskFlow::Retry { err_message }
            | TaskFlow::Dead { err_message } => {
                return Err(BaseError::Unrecoverable {
                    message: err_message,
                });
            }
        }
    }

    accept(())
}
