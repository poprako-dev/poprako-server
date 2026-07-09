//! Prom (promise) port for deferred actions.
//!
//! Prom records are enqueued during a transaction and processed after the
//! transaction commits. This allows side-effects that must not run inside
//! the transaction — such as deleting old avatar files from object storage
//! or checking whether an upload completed — to be scheduled atomically
//! with the state change that triggers them.
//!
//! # Pattern
//!
//! 1. During a [`Drive::with_context`] block, use [`PromStep::append`]
//!    to enqueue an [`Append`] step with a [`Payload`] and a `visible_at`
//!    time.
//! 2. After the transaction commits, a background worker processes the
//!    prom table, executing the deferred actions once their `visible_at`
//!    timestamp has passed.
//!
//! [`Drive::with_context`]: poprako_transactional::drive::Drive::with_context

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;
use poprako_transactional::step::Step;

use crate::part::prom::task::{ComicTask, ImageTask};
use crate::result::RegularError;

/// Prom task type definitions.
pub mod task;

/// A serializable deferred-action payload.
///
/// Currently only carries [`ImageTask`] and [`ComicArchiveTask`] variants.
/// Additional intention types can be added as new enum variants.
///
/// All data is borrowed — no heap allocation on the append path.
#[cfg_attr(test, derive(Debug, Clone, PartialEq, Eq))]
#[derive(Serialize, Deserialize)]
pub enum Payload<'a> {
    #[serde(borrow)]
    Image(ImageTask<'a>),
    #[serde(borrow)]
    Comic(ComicTask<'a>),
}

/// A [`Step`] that appends a deferred-action record.
///
/// Enqueued during a transaction via [`PromStep::append`]. The prom worker
/// will not process this record until `visible_at` has passed.
pub struct Append<'a> {
    pub id: &'a str,

    pub topic: &'a str,
    pub payload: Payload<'a>,

    pub visible_at: &'a OffsetDateTime,
}

impl<'a> Step for Append<'a> {
    type Output = ();
}

/// Factory for constructing [`Append`] steps.
pub struct PromStep;

impl PromStep {
    /// Constructs an [`Append`] step that enqueues a deferred action.
    pub fn append<'a>(
        id: &'a str,
        topic: &'a str,
        payload: Payload<'a>,
        visible_at: &'a OffsetDateTime,
    ) -> Append<'a> {
        Append {
            id,
            topic,
            payload,
            visible_at,
        }
    }
}

/// Transactional prom trait — can [`Advance`] an [`Append`] step.
///
/// This is the trait that the transactional handle must implement to
/// support enqueuing deferred actions within a transaction.
pub trait Prom<C>:
    for<'a> Advance<Append<'a>, C, Error = RegularError>
{
}
