// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// use diesel::prelude::*;
// use diesel::sql_types::{BigInt, Jsonb, Nullable, Text, Timestamptz};
// use serde_json::Value;
// use time::OffsetDateTime;
// 
// use crate::domain::model::aggr::local_message::{
//     LocalMessageAggr, LocalMessageForm, LocalMessageStatus,
// };
// use crate::domain::result::{DomainError, DomainResult};
// use crate::infra::repo::schema;
// 
// #[derive(Queryable, QueryableByName, Selectable)]
// #[diesel(table_name = schema::t_local_message)]
// pub struct LocalMessageRow {
//     #[diesel(sql_type = Text)]
//     pub f_id: String,
//     #[diesel(sql_type = Text)]
//     pub f_topic: String,
//     #[diesel(sql_type = Text)]
//     pub f_status: String,
//     #[diesel(sql_type = Jsonb)]
//     pub f_payload: Value,
//     #[diesel(sql_type = Nullable<Text>)]
//     pub f_last_error: Option<String>,
//     #[diesel(sql_type = BigInt)]
//     pub f_retried_count: i64,
//     #[diesel(sql_type = BigInt)]
//     pub f_lease: i64,
//     #[diesel(sql_type = Timestamptz)]
//     pub f_visible_at: OffsetDateTime,
//     #[diesel(sql_type = Timestamptz)]
//     pub f_created_at: OffsetDateTime,
//     #[diesel(sql_type = Timestamptz)]
//     pub f_updated_at: OffsetDateTime,
// }
// 
// #[derive(Insertable)]
// #[diesel(table_name = schema::t_local_message)]
// pub struct LocalMessageEntry<'a> {
//     pub f_id: &'a str,
//     pub f_topic: &'a str,
//     pub f_status: &'a str,
//     pub f_payload: &'a Value,
//     pub f_visible_at: OffsetDateTime,
//     pub f_created_at: OffsetDateTime,
//     pub f_updated_at: OffsetDateTime,
// }
// 
// impl<'a> LocalMessageEntry<'a> {
//     pub fn from_form(form: &'a LocalMessageForm, now: OffsetDateTime) -> Self {
//         Self {
//             f_id: &form.id,
//             f_topic: &form.topic,
//             f_status: LocalMessageStatus::Pending.as_str(),
//             f_payload: &form.payload,
//             f_visible_at: form.visible_at,
//             f_created_at: now,
//             f_updated_at: now,
//         }
//     }
// }
// 
// #[derive(AsChangeset)]
// #[diesel(table_name = schema::t_local_message)]
// pub struct LocalMessageAspect<'a> {
//     pub f_status: Option<&'a str>,
//     pub f_last_error: Option<Option<&'a str>>,
//     pub f_retried_count: Option<i64>,
//     pub f_lease: Option<i64>,
//     pub f_visible_at: Option<OffsetDateTime>,
//     pub f_updated_at: OffsetDateTime,
// }
// 
// impl<'a> LocalMessageAspect<'a> {
//     pub fn new(updated_at: OffsetDateTime) -> Self {
//         Self {
//             f_status: None,
//             f_last_error: None,
//             f_retried_count: None,
//             f_lease: None,
//             f_visible_at: None,
//             f_updated_at: updated_at,
//         }
//     }
// 
//     pub fn status(mut self, val: &'a str) -> Self {
//         self.f_status = Some(val);
//         self
//     }
// 
//     pub fn last_error(mut self, val: Option<&'a str>) -> Self {
//         self.f_last_error = Some(val);
//         self
//     }
// 
//     pub fn retried_count(mut self, val: i64) -> Self {
//         self.f_retried_count = Some(val);
//         self
//     }
// 
//     pub fn lease(mut self, val: i64) -> Self {
//         self.f_lease = Some(val);
//         self
//     }
// 
//     pub fn visible_at(mut self, val: OffsetDateTime) -> Self {
//         self.f_visible_at = Some(val);
//         self
//     }
// }
// 
// impl TryFrom<LocalMessageRow> for LocalMessageAggr {
//     type Error = DomainError;
// 
//     fn try_from(val: LocalMessageRow) -> DomainResult<Self> {
//         let status = match val.f_status.as_str() {
//             "local_message_status:pending" => LocalMessageStatus::Pending,
//             "local_message_status:processing" => LocalMessageStatus::Processing,
//             "local_message_status:completed" => LocalMessageStatus::Completed,
//             "local_message_status:dead" => LocalMessageStatus::Dead,
//             _ => {
//                 return Err(DomainError::unrecoverable(format!(
//                     "[LocalMessageRow::try_from] invalid local message status: {}",
//                     val.f_status
//                 )));
//             }
//         };
// 
//         Ok(LocalMessageAggr {
//             id: val.f_id,
//             topic: val.f_topic,
//             status,
//             payload: val.f_payload,
//             last_error: val.f_last_error,
//             retried_count: val.f_retried_count,
//             lease: val.f_lease,
//             visible_at: val.f_visible_at,
//             created_at: val.f_created_at,
//             updated_at: val.f_updated_at,
//         })
//     }
// }
