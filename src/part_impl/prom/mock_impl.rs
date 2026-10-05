#![allow(
    clippy::expect_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

//! Mock deferred-task recording and on-demand processing.

mod defer;
mod json;

#[cfg(test)]
mod tests;

use time::OffsetDateTime;

use poprako_prom::general::dispatch_flow::DispatchFlow;

use crate::part::prom::payload::PromPayload;
use crate::part_impl::prom::dispatch;
use crate::part_impl::repo::mock_impl::Mock;
use crate::part_impl::repo::mock_impl::MockContext;
use crate::result::{BaseError, BaseRest, accept};

/// One deferred action recorded by the mock transaction context.
#[cfg_attr(test, derive(Clone))]
pub struct MockPromRecord {
    pub(super) id: String,
    pub(super) payload_json: String,
    pub(super) visible_at: OffsetDateTime,
    pub(super) created_at: OffsetDateTime,
}

impl MockPromRecord {
    /// Returns the durable message identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the first processing time.
    #[must_use]
    pub fn visible_at(&self) -> OffsetDateTime {
        self.visible_at
    }

    /// Decodes the stored payload for assertions and processing.
    /// # Panics
    /// Panics if the recorded payload is invalid JSON or has an invalid shape.
    #[must_use]
    pub fn payload(&self) -> PromPayload {
        serde_json::from_str(&self.payload_json)
            .expect("stored prom payload should deserialize successfully")
    }
}

/// Processes every recorded non-object deferred action.
/// # Errors
/// Returns an unrecoverable error when dispatch requests retry or dead-letter handling.
pub async fn process_pending(mock: &Mock) -> BaseRest<()> {
    let snapshot = mock.snapshot();

    for record in &snapshot.prom_records {
        let flow = dispatch::dispatch::<MockContext, _, _, _, _>(
            (mock, mock, mock, mock),
            record.payload(),
        )
        .await
        .limit_wait(record.created_at, OffsetDateTime::now_utc());

        match flow {
            DispatchFlow::Complete | DispatchFlow::Wait { .. } => {}

            DispatchFlow::Retry { err_msg }
            | DispatchFlow::Dead { err_msg } => {
                return Err(BaseError::Unrecoverable { msg: err_msg });
            }
        }
    }

    accept(())
}
