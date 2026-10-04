/// Expands a Send delivery adapter around a concrete transaction coordinator.
#[doc(hidden)]
#[macro_export]
// Expands a Send delivery adapter around a concrete transaction coordinator.
macro_rules! __general_prom_delivery {
    ($delivery:ident, $repo:ident, $nucl:ty, $claim_level:ty, $error:ty) => {
        /// Consumer delivery bound to the application's task coordinator.
        pub mod delivery {
            use poprako_orchestra::{Nucl as _, OperStep as _};
            use time::OffsetDateTime;

            use super::repo::{
                ClaimPending, CompleteTask, FailTask, PurgeDead, ResetStuck,
                RetryTask, $repo,
            };
            use $crate::general::delivery::{ClaimedTask, Delivery};

            /// Task delivery composed from a serializable coordinator and storage.
            #[derive(Clone)]
            pub struct $delivery {
                nucl: $nucl,
                repo: $repo,
            }

            impl $delivery {
                /// Constructs a delivery without acquiring a connection.
                pub const fn new(nucl: $nucl, repo: $repo) -> Self {
                    Self { nucl, repo }
                }
            }

            impl Delivery for $delivery {
                type Error = $error;

                async fn claim(
                    &self,
                    topic: &str,
                ) -> Result<Option<ClaimedTask>, Self::Error> {
                    let row = self
                        .nucl
                        .coord(async |context| {
                            let context: &mut poprako_rdb_core::RdbContext<
                                $claim_level,
                            > = context;

                            ClaimPending::new(topic)
                                .step_on(&self.repo, context)
                                .await
                        })
                        .await?;

                    Ok(row)
                }

                async fn complete(
                    &self,
                    task: &ClaimedTask,
                ) -> Result<(), Self::Error> {
                    self.nucl
                        .coord(async |context| {
                            CompleteTask::new(task.id(), task.claim_token())
                                .step_on(&self.repo, context)
                                .await
                        })
                        .await?;

                    Ok(())
                }

                async fn retry(
                    &self,
                    task: &ClaimedTask,
                    msg: &str,
                    visible_at: OffsetDateTime,
                    retry_delta: i64,
                ) -> Result<(), Self::Error> {
                    self.nucl
                        .coord(async |context| {
                            RetryTask::new(
                                task.id(),
                                task.claim_token(),
                                msg,
                                &visible_at,
                                retry_delta,
                            )
                            .step_on(&self.repo, context)
                            .await
                        })
                        .await?;

                    Ok(())
                }

                async fn dead(
                    &self,
                    task: &ClaimedTask,
                    msg: &str,
                ) -> Result<(), Self::Error> {
                    self.nucl
                        .coord(async |context| {
                            FailTask::new(task.id(), task.claim_token(), msg)
                                .step_on(&self.repo, context)
                                .await
                        })
                        .await?;

                    Ok(())
                }

                async fn reset_stuck(
                    &self,
                    topic: &str,
                    before: OffsetDateTime,
                ) -> Result<(), Self::Error> {
                    self.nucl
                        .coord(async |context| {
                            ResetStuck::new(topic, &before)
                                .step_on(&self.repo, context)
                                .await
                        })
                        .await?;

                    Ok(())
                }

                async fn purge_dead(
                    &self,
                    topic: &str,
                    before: OffsetDateTime,
                ) -> Result<usize, Self::Error> {
                    let count = self
                        .nucl
                        .coord(async |context| {
                            PurgeDead::new(topic, &before)
                                .step_on(&self.repo, context)
                                .await
                        })
                        .await?;

                    Ok(count)
                }
            }
        }
    };
}
