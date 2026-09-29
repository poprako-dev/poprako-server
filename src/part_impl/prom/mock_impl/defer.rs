use poprako_orchestra::{OperStep as _, Step};
use poprako_orchestra_extra::prom::Prom;
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use poprako_orchestra_extra::prom::task::Task;
use time::OffsetDateTime;

use crate::part::nucl::ReptRead;
use crate::part::prom::payload::TaskPayload;
use crate::part_impl::prom::mock_impl::json::serialize_payload_err;
use crate::part_impl::prom::mock_impl::{Mock, MockPromRecord};
use crate::part_impl::repo::mock_impl::MockContext;
use crate::result::{BaseError, accept};

impl Prom<MockContext, String, TaskPayload> for Mock {
    // Defines the adapter error exposed by this producer.
    type Error = BaseError;

    // Single-task persistence has no output.
    type IndivOutput = ();

    // Batch persistence has no output.
    type BatchOutput = ();
}

/// Defers one record in the coordinated mock state.
impl<'a> Step<Defer<'a, String, TaskPayload, ()>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &Defer<'a, String, TaskPayload, ()>,
    ) -> Result<(), Self::Error> {
        //
        // Internal implementation detail.
        let payload_json = serde_json::to_string(oper.task.payload)
            .map_err(serialize_payload_err)?;

        let now = OffsetDateTime::now_utc();

        context.state.prom_records.push(MockPromRecord {
            id: oper.task.id.clone(),
            payload_json,
            visible_at: now + oper.task.delay.unwrap_or_default(),
            created_at: now,
        });

        accept(())
    }
}

impl<'t, 'a> Step<DeferBatch<'t, 'a, String, TaskPayload, ()>, MockContext>
    for Mock
{
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeferBatch<'t, 'a, String, TaskPayload, ()>,
    ) -> Result<(), Self::Error> {
        //
        // Internal implementation detail.
        for task in oper.tasks {
            //
            Defer::new(Task {
                id: task.id,
                payload: task.payload,
                delay: task.delay,
            })
            .step_on(self, context)
            .await?;
        }

        accept(())
    }
}
