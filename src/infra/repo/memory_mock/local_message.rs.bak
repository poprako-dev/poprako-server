// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// use async_trait::async_trait;
// use time::OffsetDateTime;
// 
// use poprako_util::page::Page;
// 
// use crate::domain::model::aggr::local_message::{
//     LocalMessageAggr, LocalMessageForm, LocalMessageMark, LocalMessageStatus,
// };
// use crate::domain::repo_legacy::local_message::{
//     LocalMessageQuery, LocalMessageRepoTransactional,
// };
// use crate::domain::result::{DomainError, DomainResult};
// use crate::infra::repo::memory_mock::{MemoryMockQuery, MemoryMockRepoTransactional};
// 
// fn stale_mark_error(id: &str, lease: i64) -> DomainError {
//     DomainError::unrecoverable(format!(
//         "[LocalMessageQuery::mark] stale local message mark: id={}, lease={}",
//         id, lease
//     ))
// }
// 
// #[async_trait]
// impl LocalMessageQuery for MemoryMockQuery {
//     async fn claim(&self, topic: &str, limit: i64) -> DomainResult<Vec<LocalMessageAggr>> {
//         let now = OffsetDateTime::now_utc();
//         let mut state = self.state.lock().unwrap();
//         let mut indexes: Vec<usize> = state
//             .local_messages
//             .iter()
//             .enumerate()
//             .filter(|(_, m)| m.topic == topic)
//             .filter(|(_, m)| m.status == LocalMessageStatus::Pending)
//             .filter(|(_, m)| m.visible_at <= now)
//             .map(|(index, _)| index)
//             .collect();
// 
//         indexes.sort_by_key(|index| state.local_messages[*index].created_at);
//         indexes.truncate(limit as usize);
// 
//         let mut result = Vec::with_capacity(indexes.len());
//         for index in indexes {
//             let m = &mut state.local_messages[index];
//             m.status = LocalMessageStatus::Processing;
//             m.last_error = None;
//             m.lease += 1;
//             m.updated_at = now;
//             result.push(m.clone());
//         }
// 
//         Ok(result)
//     }
// 
//     async fn mark(&self, marks: &[&LocalMessageMark]) -> DomainResult<()> {
//         let mut state = self.state.lock().unwrap();
// 
//         for mark in marks {
//             match mark {
//                 LocalMessageMark::Pending {
//                     id,
//                     lease,
//                     next_visible_at,
//                     last_error,
//                 } => {
//                     let m = state
//                         .local_messages
//                         .iter_mut()
//                         .find(|m| {
//                             m.id == *id
//                                 && m.status == LocalMessageStatus::Processing
//                                 && m.lease == *lease
//                         })
//                         .ok_or_else(|| stale_mark_error(id, *lease))?;
// 
//                     m.status = LocalMessageStatus::Pending;
//                     m.last_error = Some(last_error.clone());
//                     m.retried_count += 1;
//                     m.visible_at = *next_visible_at;
//                     m.updated_at = OffsetDateTime::now_utc();
//                 }
//                 LocalMessageMark::Completed { id, lease } => {
//                     let m = state
//                         .local_messages
//                         .iter_mut()
//                         .find(|m| {
//                             m.id == *id
//                                 && m.status == LocalMessageStatus::Processing
//                                 && m.lease == *lease
//                         })
//                         .ok_or_else(|| stale_mark_error(id, *lease))?;
// 
//                     m.status = LocalMessageStatus::Completed;
//                     m.last_error = None;
//                     m.updated_at = OffsetDateTime::now_utc();
//                 }
//                 LocalMessageMark::Dead {
//                     id,
//                     lease,
//                     last_error,
//                 } => {
//                     let m = state
//                         .local_messages
//                         .iter_mut()
//                         .find(|m| {
//                             m.id == *id
//                                 && m.status == LocalMessageStatus::Processing
//                                 && m.lease == *lease
//                         })
//                         .ok_or_else(|| stale_mark_error(id, *lease))?;
// 
//                     m.status = LocalMessageStatus::Dead;
//                     m.last_error = Some(last_error.clone());
//                     m.updated_at = OffsetDateTime::now_utc();
//                 }
//             }
//         }
// 
//         Ok(())
//     }
// 
//     async fn list_dead(&self, topic: &str, page: Page) -> DomainResult<Vec<LocalMessageAggr>> {
//         let state = self.state.lock().unwrap();
//         let mut msgs: Vec<LocalMessageAggr> = state
//             .local_messages
//             .iter()
//             .filter(|m| m.topic == topic)
//             .filter(|m| m.status == LocalMessageStatus::Dead)
//             .cloned()
//             .collect();
// 
//         msgs.sort_by_key(|m| m.updated_at);
//         msgs.reverse();
// 
//         Ok(msgs
//             .into_iter()
//             .skip(page.offset)
//             .take(page.limit)
//             .collect())
//     }
// 
//     async fn purge_completed(&self, topic: &str) -> DomainResult<()> {
//         let mut state = self.state.lock().unwrap();
//         state
//             .local_messages
//             .retain(|m| m.topic != topic || m.status != LocalMessageStatus::Completed);
// 
//         Ok(())
//     }
// 
//     async fn delete_dead(&self, items: &[&str]) -> DomainResult<()> {
//         let mut state = self.state.lock().unwrap();
//         state.local_messages.retain(|m| {
//             !items.iter().any(|id| m.id == **id) || m.status != LocalMessageStatus::Dead
//         });
// 
//         Ok(())
//     }
// }
// 
// #[async_trait]
// impl LocalMessageRepoTransactional for MemoryMockRepoTransactional {
//     async fn append(&mut self, form: &LocalMessageForm) -> DomainResult<LocalMessageAggr> {
//         let now = OffsetDateTime::now_utc();
//         let m = LocalMessageAggr {
//             id: form.id.clone(),
//             topic: form.topic.clone(),
//             status: LocalMessageStatus::Pending,
//             payload: form.payload.clone(),
//             last_error: None,
//             retried_count: 0,
//             lease: 0,
//             visible_at: form.visible_at,
//             created_at: now,
//             updated_at: now,
//         };
// 
//         let mut state = self.state.lock().unwrap();
//         state.local_messages.push(m.clone());
// 
//         Ok(m)
//     }
// 
//     async fn mark_transactional(&mut self, marks: &[&LocalMessageMark]) -> DomainResult<()> {
//         let repo = MemoryMockQuery {
//             state: self.state.clone(),
//         };
// 
//         LocalMessageQuery::mark(&repo, marks).await
//     }
// }
// 
// #[cfg(test)]
// mod tests {
//     // append_then_claim_sets_processing_and_increments_lease(LocalMessageRepoTransactional::append/LocalMessageQuery::claim)(positive): claiming an appended message should set processing and increment lease.
//     // mark_completed_with_stale_lease_returns_error(LocalMessageQuery::mark)(negative): stale worker completion should fail without changing the message.
//     // mark_pending_records_retry_metadata(LocalMessageQuery::mark)(positive): failed processing should become pending with retry metadata.
//     // mark_dead_lists_and_delete_dead_removes(LocalMessageQuery::mark/LocalMessageQuery::list_dead/LocalMessageQuery::delete_dead)(positive): dead messages should be listed and manually removable.
//     // purge_completed_removes_only_completed(LocalMessageQuery::purge_completed)(positive): purging should delete completed messages while preserving other states.
// 
//     use super::*;
// 
//     use futures_util::FutureExt as _;
// 
//     use serde_json::json;
// 
//     use poprako_util::page::Page;
// 
//     use crate::domain::model::aggr::local_message::{
//         LocalMessageForm, LocalMessageMark, LocalMessageStatus,
//     };
//     use crate::domain::repo_legacy::Transactional;
//     use crate::domain::repo_legacy::local_message::{
//         LocalMessageQuery, LocalMessageRepoTransactional,
//     };
//     use crate::infra::repo::memory_mock::MemoryMockQuery;
// 
//     fn form(id: &str, topic: &str) -> LocalMessageForm {
//         LocalMessageForm {
//             id: id.to_string(),
//             topic: topic.to_string(),
//             payload: json!({ "id": id }),
//             visible_at: OffsetDateTime::now_utc(),
//         }
//     }
// 
//     async fn append(mock: &MemoryMockQuery, form: LocalMessageForm) {
//         Transactional::transaction_scoped(mock, |repo| {
//             async move {
//                 LocalMessageRepoTransactional::append(repo, &form).await?;
//                 Ok(())
//             }
//             .boxed()
//         })
//         .await
//         .unwrap();
//     }
// 
//     #[tokio::test]
//     async fn append_then_claim_sets_processing_and_increments_lease() {
//         let mock = MemoryMockQuery::new();
//         append(&mock, form("local_message-1", "oss")).await;
// 
//         let claimed = LocalMessageQuery::claim(&mock, "oss", 10).await.unwrap();
// 
//         assert_eq!(claimed.len(), 1);
//         assert_eq!(claimed[0].id, "local_message-1");
//         assert_eq!(claimed[0].status, LocalMessageStatus::Processing);
//         assert_eq!(claimed[0].lease, 1);
// 
//         let snapshot = mock.snapshot();
//         assert_eq!(
//             snapshot.local_messages[0].status,
//             LocalMessageStatus::Processing
//         );
//         assert_eq!(snapshot.local_messages[0].lease, 1);
//     }
// 
//     #[tokio::test]
//     async fn mark_completed_with_stale_lease_returns_error() {
//         let mock = MemoryMockQuery::new();
//         append(&mock, form("local_message-1", "oss")).await;
//         LocalMessageQuery::claim(&mock, "oss", 1).await.unwrap();
// 
//         let mark = LocalMessageMark::Completed {
//             id: "local_message-1".to_string(),
//             lease: 0,
//         };
// 
//         let err = LocalMessageQuery::mark(&mock, &[&mark])
//             .await
//             .err()
//             .unwrap();
// 
//         assert!(matches!(err, DomainError::Unrecoverable { .. }));
//         assert_eq!(
//             mock.snapshot().local_messages[0].status,
//             LocalMessageStatus::Processing
//         );
//     }
// 
//     #[tokio::test]
//     async fn mark_pending_records_retry_metadata() {
//         let mock = MemoryMockQuery::new();
//         append(&mock, form("local_message-1", "oss")).await;
//         let claimed = LocalMessageQuery::claim(&mock, "oss", 1).await.unwrap();
//         let next_visible_at = OffsetDateTime::now_utc();
//         let mark = LocalMessageMark::Pending {
//             id: claimed[0].id.clone(),
//             lease: claimed[0].lease,
//             next_visible_at,
//             last_error: "failed".to_string(),
//         };
// 
//         LocalMessageQuery::mark(&mock, &[&mark]).await.unwrap();
// 
//         let m = &mock.snapshot().local_messages[0];
//         assert_eq!(m.status, LocalMessageStatus::Pending);
//         assert_eq!(m.retried_count, 1);
//         assert_eq!(m.last_error.as_deref(), Some("failed"));
//     }
// 
//     #[tokio::test]
//     async fn mark_dead_lists_and_delete_dead_removes() {
//         let mock = MemoryMockQuery::new();
//         append(&mock, form("local_message-1", "oss")).await;
//         let claimed = LocalMessageQuery::claim(&mock, "oss", 1).await.unwrap();
//         let mark = LocalMessageMark::Dead {
//             id: claimed[0].id.clone(),
//             lease: claimed[0].lease,
//             last_error: "dead".to_string(),
//         };
// 
//         LocalMessageQuery::mark(&mock, &[&mark]).await.unwrap();
// 
//         let dead = LocalMessageQuery::list_dead(
//             &mock,
//             "oss",
//             Page {
//                 offset: 0,
//                 limit: 10,
//             },
//         )
//         .await
//         .unwrap();
//         assert_eq!(dead.len(), 1);
// 
//         LocalMessageQuery::delete_dead(&mock, &[&dead[0].id])
//             .await
//             .unwrap();
//         assert!(mock.snapshot().local_messages.is_empty());
//     }
// 
//     #[tokio::test]
//     async fn purge_completed_removes_only_completed() {
//         let mock = MemoryMockQuery::new();
//         append(&mock, form("local_message-1", "oss")).await;
//         append(&mock, form("local_message-2", "oss")).await;
//         let claimed = LocalMessageQuery::claim(&mock, "oss", 2).await.unwrap();
//         let completed = LocalMessageMark::Completed {
//             id: claimed[0].id.clone(),
//             lease: claimed[0].lease,
//         };
//         let dead = LocalMessageMark::Dead {
//             id: claimed[1].id.clone(),
//             lease: claimed[1].lease,
//             last_error: "dead".to_string(),
//         };
//         LocalMessageQuery::mark(&mock, &[&completed, &dead])
//             .await
//             .unwrap();
// 
//         LocalMessageQuery::purge_completed(&mock, "oss")
//             .await
//             .unwrap();
// 
//         let snapshot = mock.snapshot();
//         assert_eq!(snapshot.local_messages.len(), 1);
//         assert_eq!(snapshot.local_messages[0].status, LocalMessageStatus::Dead);
//     }
// }
