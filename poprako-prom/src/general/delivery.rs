use std::fmt::Debug;
use std::future::Future;

#[cfg(feature = "rdb_impl")]
use diesel::Queryable;

use time::OffsetDateTime;
use uuid::Uuid;

/// An exclusively claimed attempt and the metadata needed for delivery policy.
#[derive(Debug)]
#[cfg_attr(feature = "rdb_impl", derive(Queryable))]
pub struct ClaimedTask {
    /// Stable message identity.
    id: String,

    /// Persisted routing topic.
    topic: String,
    /// Serialized business payload, decoded through `Handler::Payload`.
    payload: serde_json::Value,

    /// Number of consumed failure retries.
    retried_count: i64,
    /// Credential fencing all writes for this attempt.
    claim_token: Uuid,

    /// Original request time, retained across retries and waits.
    created_at: OffsetDateTime,
}

impl ClaimedTask {
    /// Constructs one claimed attempt with its persisted delivery metadata.
    #[must_use]
    pub fn new(
        task: (String, String, serde_json::Value),
        attempt: (i64, Uuid, OffsetDateTime),
    ) -> Self {
        //
        let (id, topic, payload) = task;

        let (retried_count, claim_token, created_at) = attempt;

        Self {
            id,
            topic,
            payload,
            retried_count,
            claim_token,
            created_at,
        }
    }

    /// Returns the stable task identity.
    #[must_use]
    pub const fn id(&self) -> &str {
        self.id.as_str()
    }

    /// Returns the persisted routing topic.
    #[must_use]
    pub const fn topic(&self) -> &str {
        self.topic.as_str()
    }

    /// Returns the serialized business payload.
    #[must_use]
    pub const fn payload(&self) -> &serde_json::Value {
        &self.payload
    }

    /// Returns the consumed failure retry count.
    #[must_use]
    pub const fn retried_count(&self) -> i64 {
        self.retried_count
    }

    /// Returns the fencing credential for this attempt.
    #[must_use]
    pub const fn claim_token(&self) -> Uuid {
        self.claim_token
    }

    /// Returns the original request time.
    #[must_use]
    pub const fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

/// Delivery access for the task lifecycle, independent of business delivery.
///
/// Implementations claim at most one active attempt per topic across consumers.
/// Finalization must match both message identity and claim token. Stale writes are
/// successful no-ops. Each operation owns its delivery transaction, separately from
/// any transaction performed by the handler.
pub trait Delivery {
    /// Infrastructure failure with safe diagnostics for consumed failures.
    type Error: Debug;

    /// Atomically claims the oldest visible message in an idle topic.
    fn claim(
        &self,
        topic: &str,
    ) -> impl Future<Output = Result<Option<ClaimedTask>, Self::Error>> + Send;

    /// Deletes the message owned by a successful attempt.
    fn complete(
        &self,
        task: &ClaimedTask,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Reschedules an attempt; `retry_delta` is one for failure and zero for wait.
    fn retry(
        &self,
        task: &ClaimedTask,
        msg: &str,
        visible_at: OffsetDateTime,
        retry_delta: i64,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Terminates an attempt while retaining its diagnostic.
    fn dead(
        &self,
        task: &ClaimedTask,
        msg: &str,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Recovers expired attempts within one topic, consuming the retry budget.
    fn reset_stuck(
        &self,
        topic: &str,
        before: OffsetDateTime,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Deletes dead records older than the cutoff within one topic.
    fn purge_dead(
        &self,
        topic: &str,
        before: OffsetDateTime,
    ) -> impl Future<Output = Result<usize, Self::Error>> + Send;
}
