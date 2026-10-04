/// Expands the typed entry and transactional producer for one table.
#[doc(hidden)]
#[macro_export]
// Expands transactional deferred-task writers.
macro_rules! __general_prom_writer {
    ($writer:ident, $table:ident, $write_level:ty, $error:ty) => {
        /// Schema-bound insertion values and claimed attempts.
        pub mod entity {
            use diesel::Insertable;
            use time::OffsetDateTime;
            use uuid::Uuid;

            use $crate::general::handler::Payload;
            use poprako_orchestra_extra::prom::task::Task;
            use $crate::general::rdb_impl::LocalTaskStatus;
            use $crate::general::rdb_impl::PromError;
            use super::$table;

            /// Insertable values for the bound task table.
            #[derive(Insertable)]
            #[diesel(table_name = $table)]
            pub struct LocalTaskEntryRow<'a> {
                /// Stable task identity supplied by its producer.
                pub f_id: &'a str,

                /// Topic selected by the typed payload.
                pub f_topic: &'a str,
                /// Initial lifecycle status.
                pub f_status: LocalTaskStatus,
                /// Empty until a consumer claims this record.
                pub f_claim_token: Option<Uuid>,

                /// Serialized application payload.
                pub f_payload: serde_json::Value,

                /// Earliest time at which this task can be claimed.
                pub f_visible_at: OffsetDateTime,

                /// Original request time used for the waiting deadline.
                pub f_created_at: OffsetDateTime,
                /// Time of the most recent lifecycle transition.
                pub f_updated_at: OffsetDateTime,
            }

            impl<'a> LocalTaskEntryRow<'a> {
                /// Serializes one task and checks its visibility timestamp.
                pub fn from_task<P: Payload>(
                    task: &Task<'a, String, P>,
                    now: OffsetDateTime,
                ) -> Result<Self, PromError> {
                    //
                    let payload =
                        serde_json::to_value(task.payload).map_err(|err_serde| {
                            //
                            tracing::error!(
                                operation = "serialize_prom_payload",
                                sdk_err = ?err_serde,
                                "JSON SDK serialization error",
                            );

                            PromError::Unrecoverable {
                                msg: format!(
                                    "failed to serialize prom payload: {}",
                                    err_serde,
                                ),
                            }
                        })?;

                    let delay = time::Duration::try_from(task.delay.unwrap_or_default())
                        .map_err(|_| PromError::Unrecoverable {
                            msg: "prom task delay is outside the supported range"
                                .to_string(),
                        })?;

                    let visible_at =
                        now.checked_add(delay)
                            .ok_or_else(|| PromError::Unrecoverable {
                                msg:
                                    "prom task visibility is outside the supported range"
                                        .to_string(),
                            })?;

                    Ok(Self {
                        f_id: task.id.as_ref(),
                        f_topic: task.payload.topic(),
                        f_status: LocalTaskStatus::Pending,
                        f_claim_token: None,
                        f_payload: payload,
                        f_visible_at: visible_at,
                        f_created_at: now,
                        f_updated_at: now,
                    })
                }
            }

        }

        /// Transactional producer bound to the application's task table.
        pub mod writer {
            use diesel_async::RunQueryDsl as _;
            use poprako_orchestra::{AtLeast, Level, Step};
            use poprako_rdb_core::{RdbConn, RdbContext};
            use time::OffsetDateTime;
            use tracing::instrument;

            use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
            use $crate::general::handler::Payload;
            use poprako_orchestra_extra::prom::Prom;
            use poprako_orchestra_extra::prom::task::Task;
            use super::entity::LocalTaskEntryRow;
            use super::$table;

            type WriteLevel = $write_level;
            type AppError = $error;
            type AppRest<T> = Result<T, AppError>;

            fn database_error(error: diesel::result::Error) -> AppError {
                $crate::general::rdb_impl::diesel_error(error).into()
            }

            /// Transactional deferred-task writer, independent of background consumption.
            #[derive(Clone, Copy, Default)]
            pub struct $writer;

            impl $writer {
                /// Constructs a writer without starting a consumer.
                #[must_use]
                pub const fn new() -> Self {
                    Self
                }
            }

            impl<L, P> Prom<RdbContext<L>, String, P> for $writer
            where
                L: Level + Send + AtLeast<WriteLevel>,
                P: Payload + Sync,
            {
                // Defines the adapter error exposed by this producer.
                type Error = AppError;

                // Single-task delivery has no output.
                type IndivOutput = ();

                // Batch delivery has no output.
                type BatchOutput = ();
            }

            impl<'a, L, P> Step<Defer<'a, String, P, ()>, RdbContext<L>> for $writer
            where
                L: Level + Send + AtLeast<WriteLevel>,
                P: Payload + Sync,
            {
                type Level = WriteLevel;

                // Defines the adapter error exposed by this operation.
                type Error = AppError;

                #[instrument(level = "info", skip_all)]
                async fn step(
                    &self,
                    context: &mut RdbContext<L>,
                    oper: &Defer<'a, String, P, ()>,
                ) -> AppRest<()> {
                    defer(context.conn(), &oper.task).await
                }
            }

            impl<'t, 'a, L, P> Step<DeferBatch<'t, 'a, String, P, ()>, RdbContext<L>>
                for $writer
            where
                L: Level + Send + AtLeast<WriteLevel>,
                P: Payload + Sync,
            {
                type Level = WriteLevel;

                // Defines the adapter error exposed by this operation.
                type Error = AppError;

                #[instrument(level = "info", skip_all)]
                async fn step(
                    &self,
                    context: &mut RdbContext<L>,
                    oper: &DeferBatch<'t, 'a, String, P, ()>,
                ) -> AppRest<()> {
                    defer_batch(context.conn(), oper.tasks).await
                }
            }

            // Implements defer.
            #[instrument(level = "info", skip_all)]
            async fn defer<P: Payload>(
                conn: &mut RdbConn,
                task: &Task<'_, String, P>,
            ) -> AppRest<()> {
                //
                let now = OffsetDateTime::now_utc();

                let entry = LocalTaskEntryRow::from_task(task, now)?;

                diesel::insert_into($table::table)
                    .values(&entry)
                    .execute(conn)
                    .await
                    .map_err(database_error)?;

                Ok(())
            }

            // Implements defer batch.
            #[instrument(level = "info", skip_all)]
            async fn defer_batch<P: Payload>(
                conn: &mut RdbConn,
                tasks: &[Task<'_, String, P>],
            ) -> AppRest<()> {
                //
                if tasks.is_empty() {
                    return Ok(());
                }

                let now = OffsetDateTime::now_utc();

                let entries = tasks
                    .iter()
                    .map(|task| LocalTaskEntryRow::from_task(task, now))
                    .collect::<Result<Vec<_>, _>>()?;

                diesel::insert_into($table::table)
                    .values(&entries)
                    .execute(conn)
                    .await
                    .map_err(database_error)?;

                Ok(())
            }

        }
    };
}
