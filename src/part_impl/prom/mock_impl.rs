//! Mock deferred-task recording and on-demand processing.

mod chapter;
mod defer;
mod invitation;
mod json;

#[cfg(test)]
mod tests;

use time::OffsetDateTime;

use crate::part::prom::payload::TaskPayload;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::{BaseRest, accept};

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
        match record.payload() {
            TaskPayload::Chapter { payload } => {
                chapter::process(mock, &payload).await?;
            }

            TaskPayload::Invitation { payload } => {
                invitation::process(mock, &payload).await?;
            }
        }
    }

    accept(())
}
