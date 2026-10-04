/// Expands the typed delivery operations for one table.
#[doc(hidden)]
#[macro_export]
// Expands typed task lifecycle repository operations.
macro_rules! __general_prom_repo {
    ($repo:ident, $table:ident, $write_level:ty, $claim_level:ty, $error:ty) => {
        /// Typed delivery lifecycle operations.
        pub mod repo {
            use diesel::dsl::{exists, not};
            use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
            use diesel::{
                NullableExpressionMethods as _, OptionalExtension as _,
                define_sql_function,
            };
            use diesel_async::RunQueryDsl as _;
            use poprako_orchestra::{AtLeast, Level, Oper, Step};
            use time::OffsetDateTime;
            use tracing::instrument;
            use uuid::Uuid;

            use poprako_rdb_core::{RdbConn, RdbContext};

            use super::$table;
            use $crate::general::delivery::ClaimedTask;
            use $crate::general::rdb_impl::LocalTaskStatus;
            type WriteLevel = $write_level;
            type ClaimLevel = $claim_level;
            type AppError = $error;
            type AppRest<T> = Result<T, AppError>;

            fn database_error(error: diesel::result::Error) -> AppError {
                $crate::general::rdb_impl::diesel_error(error).into()
            }

            /// Atomically claims the oldest visible pending message in one topic.
            /// Serializable transactions prevent competing claims for the same topic.
            #[derive(Oper)]
            #[oper(output = Option<ClaimedTask>)]
            pub struct ClaimPending<'a> {
                /// Topic whose messages this consumer may read and claim.
                topic: &'a str,
            }

            impl<'a> ClaimPending<'a> {
                /// Reserves one attempt exclusively from the requested topic.
                pub const fn new(topic: &'a str) -> Self {
                    Self { topic }
                }
            }

            /// Deletes the record owned by a successfully completed attempt.
            #[derive(Oper)]
            #[oper(output = ())]
            pub struct CompleteTask<'a> {
                /// ID of the local-message row to mark complete.
                id: &'a str,

                /// UUID credential owned by the worker attempt.
                claim_token: Uuid,
            }

            impl<'a> CompleteTask<'a> {
                /// Builds an operation that completes the identified worker attempt.
                pub const fn new(id: &'a str, claim_token: Uuid) -> Self {
                    Self { id, claim_token }
                }
            }

            /// Mark a record as dead with an error message.
            #[derive(Oper)]
            #[oper(output = ())]
            pub struct FailTask<'a> {
                /// ID of the local-message row to mark as failed.
                id: &'a str,

                /// UUID credential owned by the worker attempt.
                claim_token: Uuid,

                /// Error description attached to the failure record.
                error: &'a str,
            }

            impl<'a> FailTask<'a> {
                /// Builds an operation that permanently fails the message identified by `id`.
                pub const fn new(
                    id: &'a str,
                    claim_token: Uuid,
                    err_msg: &'a str,
                ) -> Self {
                    Self {
                        id,
                        claim_token,
                        error: err_msg,
                    }
                }
            }

            /// Reset one failed processing attempt back to pending for a later retry.
            #[derive(Oper)]
            #[oper(output = ())]
            pub struct RetryTask<'a> {
                /// ID of the local-message row to retry.
                id: &'a str,

                /// UUID credential owned by the worker attempt.
                claim_token: Uuid,

                /// Error description logged from the previous attempt.
                error: &'a str,

                /// Timestamp after which the retry becomes visible for processing.
                visible_at: &'a OffsetDateTime,
                /// Amount consumed from the failure retry budget.
                retry_delta: i64,
            }

            impl<'a> RetryTask<'a> {
                /// Builds an operation that schedules the message identified by `id` for retry.
                pub const fn new(
                    id: &'a str,
                    claim_token: Uuid,
                    err_msg: &'a str,
                    visible_at: &'a OffsetDateTime,
                    retry_delta: i64,
                ) -> Self {
                    Self {
                        id,
                        claim_token,
                        error: err_msg,
                        visible_at,
                        retry_delta,
                    }
                }
            }

            /// Reset expired processing attempts, consuming the shared failure retry budget.
            /// The fourth failed attempt becomes dead; waiting does not consume this budget.
            #[derive(Oper)]
            #[oper(output = ())]
            pub struct ResetStuck<'a> {
                /// Topic whose expired attempts may be recovered.
                topic: &'a str,
                /// Cutoff timestamp; any record stuck in Processing before this is reset.
                before: &'a OffsetDateTime,
            }

            impl<'a> ResetStuck<'a> {
                /// Builds an operation that resets messages stuck before the cutoff.
                pub const fn new(
                    topic: &'a str,
                    before: &'a OffsetDateTime,
                ) -> Self {
                    Self { topic, before }
                }
            }

            /// Deletes dead records after their retention cutoff.
            #[derive(Oper)]
            #[oper(output = usize)]
            pub struct PurgeDead<'a> {
                /// Topic whose dead messages may be purged.
                topic: &'a str,
                /// Cutoff timestamp for dead records to purge.
                dead_before: &'a OffsetDateTime,
            }

            impl<'a> PurgeDead<'a> {
                /// Builds a dead-message purge cutoff.
                pub const fn new(
                    topic: &'a str,
                    dead_before: &'a OffsetDateTime,
                ) -> Self {
                    Self { topic, dead_before }
                }
            }

            define_sql_function! {
                /// Generates an independent credential for each claimed row.
                fn gen_random_uuid() -> diesel::sql_types::Uuid;
            }

            // Claims each attempt in one typed statement within a serializable transaction.
            #[instrument(level = "info", skip_all)]
            async fn claim_pending(
                conn: &mut RdbConn,
                topic: &str,
            ) -> AppRest<Option<ClaimedTask>> {
                //
                let now = OffsetDateTime::now_utc();

                let (pending, processing, locked) = diesel::alias!(
                    $table as pending,
                    $table as processing,
                    $table as locked,
                );

                let active_topic = processing
                    .filter(processing.field($table::f_topic).eq(topic))
                    .filter(
                        processing
                            .field($table::f_status)
                            .eq(LocalTaskStatus::Processing.as_str()),
                    );

                let first_pending = pending
                    .filter(pending.field($table::f_topic).eq(topic))
                    .filter(
                        pending
                            .field($table::f_status)
                            .eq(LocalTaskStatus::Pending.as_str()),
                    )
                    .filter(pending.field($table::f_visible_at).le(now))
                    .filter(not(exists(active_topic)))
                    .order_by((
                        pending.field($table::f_created_at),
                        pending.field($table::f_id),
                    ))
                    .limit(1)
                    .select(pending.field($table::f_id));

                let claim_ids = locked
                    .filter(locked.field($table::f_topic).eq(topic))
                    .filter(locked.field($table::f_id).eq_any(first_pending))
                    .select(locked.field($table::f_id))
                    .for_update()
                    .skip_locked();

                let row = diesel::update(
                    $table::table
                        .filter($table::f_topic.eq(topic))
                        .filter($table::f_id.eq_any(claim_ids)),
                )
                .set((
                    $table::f_status.eq(LocalTaskStatus::Processing.as_str()),
                    $table::f_claim_token.eq(gen_random_uuid().nullable()),
                    $table::f_updated_at.eq(now),
                ))
                .returning((
                    $table::f_id,
                    $table::f_topic,
                    $table::f_payload,
                    $table::f_retried_count,
                    $table::f_claim_token.assume_not_null(),
                    $table::f_created_at,
                ))
                .get_result::<ClaimedTask>(conn)
                .await
                .optional()
                .map_err(database_error)?;

                Ok(row)
            }

            // Implements complete message.
            #[instrument(level = "info", skip_all)]
            async fn complete_task(
                conn: &mut RdbConn,
                id: &str,
                claim_token: Uuid,
            ) -> AppRest<()> {
                //
                diesel::delete(
                    $table::table
                        .filter($table::f_id.eq(id))
                        .filter(
                            $table::f_status
                                .eq(LocalTaskStatus::Processing.as_str()),
                        )
                        .filter($table::f_claim_token.eq(claim_token)),
                )
                .execute(conn)
                .await
                .map_err(database_error)?;

                Ok(())
            }

            // Implements fail message.
            #[instrument(level = "info", skip_all)]
            async fn fail_task(
                conn: &mut RdbConn,
                id: &str,
                claim_token: Uuid,
                error: &str,
            ) -> AppRest<()> {
                //
                diesel::update(
                    $table::table
                        .filter($table::f_id.eq(id))
                        .filter(
                            $table::f_status
                                .eq(LocalTaskStatus::Processing.as_str()),
                        )
                        .filter($table::f_claim_token.eq(claim_token)),
                )
                .set((
                    $table::f_claim_token.eq(None::<Uuid>),
                    $table::f_status.eq(LocalTaskStatus::Dead.as_str()),
                    $table::f_last_error.eq(Some(error)),
                    $table::f_updated_at.eq(OffsetDateTime::now_utc()),
                ))
                .execute(conn)
                .await
                .map_err(database_error)?;

                Ok(())
            }

            // Implements retry message.
            #[instrument(level = "info", skip_all)]
            async fn retry_task(
                conn: &mut RdbConn,
                oper: &RetryTask<'_>,
            ) -> AppRest<()> {
                //
                diesel::update(
                    $table::table
                        .filter($table::f_id.eq(oper.id))
                        .filter(
                            $table::f_status
                                .eq(LocalTaskStatus::Processing.as_str()),
                        )
                        .filter($table::f_claim_token.eq(oper.claim_token)),
                )
                .set((
                    $table::f_claim_token.eq(None::<Uuid>),
                    $table::f_status.eq(LocalTaskStatus::Pending.as_str()),
                    $table::f_last_error.eq(Some(oper.error)),
                    $table::f_retried_count
                        .eq($table::f_retried_count + oper.retry_delta),
                    $table::f_visible_at.eq(*oper.visible_at),
                    $table::f_updated_at.eq(OffsetDateTime::now_utc()),
                ))
                .execute(conn)
                .await
                .map_err(database_error)?;

                Ok(())
            }

            // Implements reset stuck.
            #[instrument(level = "info", skip_all)]
            async fn reset_stuck(
                conn: &mut RdbConn,
                topic: &str,
                before: &OffsetDateTime,
            ) -> AppRest<()> {
                //
                diesel::update(
                    $table::table
                        .filter($table::f_topic.eq(topic))
                        .filter(
                            $table::f_status
                                .eq(LocalTaskStatus::Processing.as_str()),
                        )
                        .filter($table::f_updated_at.le(*before))
                        .filter($table::f_retried_count.ge(3)),
                )
                .set((
                    $table::f_status.eq(LocalTaskStatus::Dead.as_str()),
                    $table::f_last_error
                        .eq(Some("processing timeout exceeded")),
                    $table::f_claim_token.eq(None::<Uuid>),
                    $table::f_updated_at.eq(OffsetDateTime::now_utc()),
                ))
                .execute(conn)
                .await
                .map_err(database_error)?;

                diesel::update(
                    $table::table
                        .filter($table::f_topic.eq(topic))
                        .filter(
                            $table::f_status
                                .eq(LocalTaskStatus::Processing.as_str()),
                        )
                        .filter($table::f_updated_at.le(*before))
                        .filter($table::f_retried_count.lt(3)),
                )
                .set((
                    $table::f_status.eq(LocalTaskStatus::Pending.as_str()),
                    $table::f_last_error
                        .eq(Some("processing timeout exceeded")),
                    $table::f_retried_count.eq($table::f_retried_count + 1),
                    $table::f_claim_token.eq(None::<Uuid>),
                    $table::f_updated_at.eq(OffsetDateTime::now_utc()),
                ))
                .execute(conn)
                .await
                .map_err(database_error)?;

                Ok(())
            }

            // Deletes expired dead messages.
            #[instrument(level = "info", skip_all)]
            async fn purge_dead(
                conn: &mut RdbConn,
                topic: &str,
                dead_before: &OffsetDateTime,
            ) -> AppRest<usize> {
                //
                let purged_count = diesel::delete(
                    $table::table
                        .filter($table::f_topic.eq(topic))
                        .filter(
                            $table::f_status.eq(LocalTaskStatus::Dead.as_str()),
                        )
                        .filter($table::f_updated_at.lt(*dead_before)),
                )
                .execute(conn)
                .await
                .map_err(database_error)?;

                Ok(purged_count)
            }

            /// Typed delivery lifecycle operations used by the consumer.
            ///
            /// Provides atomic claiming, completion, failure, retry, and recovery
            /// operations for the bound task table.
            ///
            #[derive(Clone, Copy, Default)]
            pub struct $repo;

            impl $repo {
                /// Builds the local-message delivery repository.
                #[must_use]
                pub const fn new() -> Self {
                    Self
                }
            }

            impl<'a, L> Step<ClaimPending<'a>, RdbContext<L>> for $repo
            where
                L: Level + Send + AtLeast<ClaimLevel>,
            {
                type Level = ClaimLevel;

                // Defines the adapter error exposed by this operation.
                type Error = AppError;
                #[instrument(level = "info", skip_all)]
                async fn step(
                    &self,
                    context: &mut RdbContext<L>,
                    oper: &ClaimPending<'a>,
                ) -> AppRest<Option<ClaimedTask>> {
                    claim_pending(context.conn(), oper.topic).await
                }
            }

            impl<'a, L> Step<CompleteTask<'a>, RdbContext<L>> for $repo
            where
                L: Level + Send + AtLeast<WriteLevel>,
            {
                type Level = WriteLevel;

                // Defines the adapter error exposed by this operation.
                type Error = AppError;
                #[instrument(level = "info", skip_all)]
                async fn step(
                    &self,
                    context: &mut RdbContext<L>,
                    oper: &CompleteTask<'a>,
                ) -> AppRest<()> {
                    complete_task(context.conn(), oper.id, oper.claim_token)
                        .await
                }
            }

            impl<'a, L> Step<FailTask<'a>, RdbContext<L>> for $repo
            where
                L: Level + Send + AtLeast<WriteLevel>,
            {
                type Level = WriteLevel;

                // Defines the adapter error exposed by this operation.
                type Error = AppError;
                #[instrument(level = "info", skip_all)]
                async fn step(
                    &self,
                    context: &mut RdbContext<L>,
                    oper: &FailTask<'a>,
                ) -> AppRest<()> {
                    //
                    fail_task(
                        context.conn(),
                        oper.id,
                        oper.claim_token,
                        oper.error,
                    )
                    .await
                }
            }

            impl<'a, L> Step<RetryTask<'a>, RdbContext<L>> for $repo
            where
                L: Level + Send + AtLeast<WriteLevel>,
            {
                type Level = WriteLevel;

                // Defines the adapter error exposed by this operation.
                type Error = AppError;
                #[instrument(level = "info", skip_all)]
                async fn step(
                    &self,
                    context: &mut RdbContext<L>,
                    oper: &RetryTask<'a>,
                ) -> AppRest<()> {
                    retry_task(context.conn(), oper).await
                }
            }

            impl<'a, L> Step<ResetStuck<'a>, RdbContext<L>> for $repo
            where
                L: Level + Send + AtLeast<WriteLevel>,
            {
                type Level = WriteLevel;

                // Defines the adapter error exposed by this operation.
                type Error = AppError;
                #[instrument(level = "info", skip_all)]
                async fn step(
                    &self,
                    context: &mut RdbContext<L>,
                    oper: &ResetStuck<'a>,
                ) -> AppRest<()> {
                    reset_stuck(context.conn(), oper.topic, oper.before).await
                }
            }

            impl<'a, L> Step<PurgeDead<'a>, RdbContext<L>> for $repo
            where
                L: Level + Send + AtLeast<WriteLevel>,
            {
                type Level = WriteLevel;

                // Defines the adapter error exposed by this operation.
                type Error = AppError;
                #[instrument(level = "info", skip_all)]
                async fn step(
                    &self,
                    context: &mut RdbContext<L>,
                    oper: &PurgeDead<'a>,
                ) -> AppRest<usize> {
                    purge_dead(context.conn(), oper.topic, oper.dead_before)
                        .await
                }
            }
        }
    };
}
