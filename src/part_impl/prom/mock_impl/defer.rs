use poprako_orchestra::{OperStep as _, Step};
use time::OffsetDateTime;

use crate::part::prom::oper::{Defer, DeferBatch};
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::task::Task;
use crate::part_impl::prom::mock_impl::json::serialize_payload_err;
use crate::part_impl::prom::mock_impl::{Mock, MockContext, MockPromRecord};
use crate::result::{BaseError, accept};

/// Defers one record in the coordinated mock state.
impl<'a> Step<Defer<'a, String, TaskPayload, ()>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = crate::part::nucl::RepeatableRead;

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

        context.state.prom_records.push(MockPromRecord {
            id: oper.task.id.to_string(),
            payload_json,
            visible_at: OffsetDateTime::now_utc()
                + oper.task.delay.unwrap_or_default(),
        });

        accept(())
    }
}

impl<'t, 'a> Step<DeferBatch<'t, 'a, String, TaskPayload, ()>, MockContext>
    for Mock
{
    // Internal type alias for `Error`.
    type Level = crate::part::nucl::RepeatableRead;

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
