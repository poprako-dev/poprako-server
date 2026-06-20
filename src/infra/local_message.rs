// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// use std::sync::Arc;
// use std::time::Duration;
// 
// use time::OffsetDateTime;
// 
// use futures_util::FutureExt as _;
// 
// use crate::domain::external::image_pool::{ImageDelete, ImageInspect};
// use crate::domain::model::aggr::local_message::{LocalMessageAggr, LocalMessageMark};
// use crate::domain::model::value::local_message::{
//     IMAGE_TOPIC, ImageLocalMessage, ImageResourceKind,
// };
// use crate::domain::query_legacy::Transactional;
// use crate::domain::query_legacy::local_message::{
//     LocalMessageQuery, LocalMessageQueryTransactional,
// };
// use crate::domain::query_legacy::team::TeamQueryTransactional;
// use crate::domain::query_legacy::user::UserQueryTransactional;
// use crate::domain::result::{DomainError, DomainResult};
// use crate::harness::HarnessBase;
// 
// pub struct LocalMessageIngestor {
//     harness: Arc<HarnessBase>,
//     options: LocalMessageIngestorOptions,
// }
// 
// struct LocalMessageIngestorOptions {
//     poll_interval: Duration,
//     claim_limit: i64,
//     max_retry: i64,
// }
// 
// // FIXME: too large
// impl LocalMessageIngestor {
//     pub fn new(harness: Arc<HarnessBase>) -> Self {
//         Self {
//             harness,
//             options: LocalMessageIngestorOptions {
//                 poll_interval: Duration::from_secs(30),
//                 claim_limit: 50,
//                 max_retry: 10,
//             },
//         }
//     }
// 
//     pub fn run(self) {
//         tokio::spawn(async move {
//             self.ingest().await;
//         });
//     }
// 
//     async fn ingest(self) {
//         let mut interval = tokio::time::interval(self.options.poll_interval);
// 
//         loop {
//             interval.tick().await;
// 
//             if let Err(err) = self.poll_ingest().await {
//                 tracing::warn!("[LocalMessageIngestor::run_forever] {}", err);
//             }
//         }
//     }
// 
//     async fn poll_ingest(&self) -> DomainResult<()> {
//         let claimed =
//             LocalMessageQuery::claim(self.harness.as_ref(), IMAGE_TOPIC, self.options.claim_limit)
//                 .await?;
// 
//         for message in claimed {
//             self.handle_message(message).await?;
//         }
// 
//         Ok(())
//     }
// 
//     async fn handle_message(&self, message: LocalMessageAggr) -> DomainResult<()> {
//         let payload = match serde_json::from_value(message.payload.clone()) {
//             Ok(payload) => payload,
//             Err(err) => {
//                 self.mark_dead(
//                     &message,
//                     format!(
//                         "[LocalMessageIngestor::handle_message] invalid image message payload: {}",
//                         err
//                     ),
//                 )
//                 .await?;
//                 return Ok(());
//             }
//         };
// 
//         let result = match payload {
//             ImageLocalMessage::CheckUploaded {
//                 resource_kind,
//                 resource_id,
//                 object_key,
//                 image_version,
//             } => {
//                 self.handle_image_check_uploaded(
//                     &message,
//                     resource_kind,
//                     &resource_id,
//                     &object_key,
//                     image_version,
//                 )
//                 .await
//             }
//             ImageLocalMessage::Delete { object_key } => {
//                 self.handle_image_delete(&message, &object_key).await
//             }
//         };
// 
//         match result {
//             Ok(()) => Ok(()),
//             Err(err) => self.mark_retry_or_dead(&message, err.to_string()).await,
//         }
//     }
// 
//     async fn handle_image_check_uploaded(
//         &self,
//         message: &LocalMessageAggr,
//         resource_kind: ImageResourceKind,
//         resource_id: &str,
//         object_key: &str,
//         avatar_version: i64,
//     ) -> DomainResult<()> {
//         let current = self.load_current_avatar(resource_kind, resource_id).await?;
// 
//         let Some(current) = current else {
//             self.mark_completed(message).await?;
//             return Ok(());
//         };
// 
//         if current.avatar_version != avatar_version || current.uploaded {
//             self.mark_completed(message).await?;
//             return Ok(());
//         }
// 
//         if !ImageInspect::exists(self.harness.as_ref(), object_key).await? {
//             self.mark_completed(message).await?;
//             return Ok(());
//         }
// 
//         let message_id = message.id.clone();
//         let lease = message.lease;
//         let resource_id = resource_id.to_string();
//         Transactional::transaction_scoped(self.harness.as_ref(), move |query| {
//             async move {
//                 let current = match resource_kind {
//                     ImageResourceKind::UserAvatar => {
//                         let user =
//                             UserQueryTransactional::get_by_id_excluded(query, &resource_id).await?;
//                         AvatarState {
//                             avatar_version: user.avatar_version,
//                             uploaded: user.avatar_uploaded,
//                         }
//                     }
//                     ImageResourceKind::TeamAvatar => {
//                         let team =
//                             TeamQueryTransactional::get_by_id_excluded(query, &resource_id).await?;
//                         AvatarState {
//                             avatar_version: team.avatar_version,
//                             uploaded: team.avatar_uploaded,
//                         }
//                     }
//                 };
// 
//                 if current.avatar_version == avatar_version && !current.uploaded {
//                     match resource_kind {
//                         ImageResourceKind::UserAvatar => {
//                             UserQueryTransactional::mark_avatar_uploaded(
//                                 query,
//                                 &resource_id,
//                                 avatar_version,
//                             )
//                             .await?;
//                         }
//                         ImageResourceKind::TeamAvatar => {
//                             TeamQueryTransactional::mark_avatar_uploaded(
//                                 query,
//                                 &resource_id,
//                                 avatar_version,
//                             )
//                             .await?;
//                         }
//                     }
//                 }
// 
//                 let mark = LocalMessageMark::Completed {
//                     id: message_id,
//                     lease,
//                 };
//                 LocalMessageQueryTransactional::mark_transactional(query, &[&mark]).await?;
// 
//                 Ok(())
//             }
//             .boxed()
//         })
//         .await?;
// 
//         Ok(())
//     }
// 
//     async fn handle_image_delete(
//         &self,
//         message: &LocalMessageAggr,
//         object_key: &str,
//     ) -> DomainResult<()> {
//         ImageDelete::delete_batch(self.harness.as_ref(), &[object_key]).await?;
//         self.mark_completed(message).await
//     }
// 
//     async fn load_current_avatar(
//         &self,
//         resource_kind: ImageResourceKind,
//         resource_id: &str,
//     ) -> DomainResult<Option<AvatarState>> {
//         let resource_id = resource_id.to_string();
//         let result = Transactional::transaction_scoped(self.harness.as_ref(), move |query| {
//             async move {
//                 match resource_kind {
//                     ImageResourceKind::UserAvatar => {
//                         let user =
//                             UserQueryTransactional::get_by_id_excluded(query, &resource_id).await?;
//                         Ok(AvatarState {
//                             avatar_version: user.avatar_version,
//                             uploaded: user.avatar_uploaded,
//                         })
//                     }
//                     ImageResourceKind::TeamAvatar => {
//                         let team =
//                             TeamQueryTransactional::get_by_id_excluded(query, &resource_id).await?;
//                         Ok(AvatarState {
//                             avatar_version: team.avatar_version,
//                             uploaded: team.avatar_uploaded,
//                         })
//                     }
//                 }
//             }
//             .boxed()
//         })
//         .await;
// 
//         match result {
//             Ok(current) => Ok(Some(current)),
//             Err(DomainError::Expected { .. }) => Ok(None),
//             Err(err) => Err(err),
//         }
//     }
// 
//     async fn mark_completed(&self, message: &LocalMessageAggr) -> DomainResult<()> {
//         let mark = LocalMessageMark::Completed {
//             id: message.id.clone(),
//             lease: message.lease,
//         };
//         LocalMessageQuery::mark(self.harness.as_ref(), &[&mark]).await
//     }
// 
//     async fn mark_dead(&self, message: &LocalMessageAggr, last_error: String) -> DomainResult<()> {
//         let mark = LocalMessageMark::Dead {
//             id: message.id.clone(),
//             lease: message.lease,
//             last_error,
//         };
//         LocalMessageQuery::mark(self.harness.as_ref(), &[&mark]).await
//     }
// 
//     async fn mark_retry_or_dead(
//         &self,
//         message: &LocalMessageAggr,
//         last_error: String,
//     ) -> DomainResult<()> {
//         if message.retried_count >= self.options.max_retry {
//             return self.mark_dead(message, last_error).await;
//         }
// 
//         let mark = LocalMessageMark::Pending {
//             id: message.id.clone(),
//             lease: message.lease,
//             next_visible_at: OffsetDateTime::now_utc() + retry_backoff(message.retried_count),
//             last_error,
//         };
//         LocalMessageQuery::mark(self.harness.as_ref(), &[&mark]).await
//     }
// }
// 
// struct AvatarState {
//     avatar_version: i64,
//     uploaded: bool,
// }
// 
// fn retry_backoff(retried_count: i64) -> time::Duration {
//     match retried_count {
//         0 => time::Duration::minutes(1),
//         1 => time::Duration::minutes(2),
//         2 => time::Duration::minutes(4),
//         3 => time::Duration::minutes(8),
//         4 => time::Duration::minutes(16),
//         _ => time::Duration::minutes(30),
//     }
// }
