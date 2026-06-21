// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// use diesel::prelude::*;
// use time::OffsetDateTime;
// 
// use crate::domain::model::aggr::team::TeamAggr;
// use crate::infra::repo::schema;
// 
// // ── Queryable / Selectable ─────────────────────────────────────────────────
// 
// #[derive(Queryable, Selectable)]
// #[diesel(table_name = schema::t_team)]
// pub struct TeamRow {
//     pub f_id: String,
//     pub f_name: String,
//     pub f_description: Option<String>,
//     pub f_avatar_key: Option<String>,
//     pub f_avatar_uploaded: bool,
//     pub f_avatar_version: i64,
//     pub f_workset_next_index: i32,
//     pub f_created_at: OffsetDateTime,
//     pub f_updated_at: OffsetDateTime,
// }
// 
// // ── Insertable ─────────────────────────────────────────────────────────────
// 
// #[derive(Insertable)]
// #[diesel(table_name = schema::t_team)]
// pub struct TeamEntry<'a> {
//     pub f_id: &'a str,
//     pub f_name: &'a str,
//     pub f_description: &'a str,
//     pub f_workset_next_index: i32,
//     pub f_created_at: OffsetDateTime,
//     pub f_updated_at: OffsetDateTime,
// }
// 
// // ── Changeset (AsChangeset) ────────────────────────────────────────────────
// 
// /// Changeset for updating team fields via partial updates.
// ///
// /// Only `Some` fields are included in the generated `SET` clause;
// /// `None` fields are omitted.
// #[derive(AsChangeset)]
// #[diesel(table_name = schema::t_team)]
// pub struct TeamAspect<'a> {
//     pub f_name: Option<&'a str>,
//     pub f_description: Option<&'a str>,
//     pub f_avatar_key: Option<&'a str>,
//     pub f_avatar_uploaded: Option<bool>,
//     pub f_avatar_version: Option<i64>,
//     pub f_updated_at: OffsetDateTime,
// }
// 
// impl<'a> TeamAspect<'a> {
//     /// Creates a new changeset with all optional fields set to `None`.
//     pub fn new(updated_at: OffsetDateTime) -> Self {
//         Self {
//             f_name: None,
//             f_description: None,
//             f_avatar_key: None,
//             f_avatar_uploaded: None,
//             f_avatar_version: None,
//             f_updated_at: updated_at,
//         }
//     }
// 
//     pub fn name(mut self, val: &'a str) -> Self {
//         self.f_name = Some(val);
//         self
//     }
// 
//     pub fn description(mut self, val: &'a str) -> Self {
//         self.f_description = Some(val);
//         self
//     }
// 
//     pub fn avatar_key(mut self, val: &'a str) -> Self {
//         self.f_avatar_key = Some(val);
//         self
//     }
// 
//     pub fn avatar_uploaded(mut self, val: bool) -> Self {
//         self.f_avatar_uploaded = Some(val);
//         self
//     }
// 
//     pub fn avatar_version(mut self, val: i64) -> Self {
//         self.f_avatar_version = Some(val);
//         self
//     }
// }
// 
// // ── Conversions ────────────────────────────────────────────────────────────
// 
// impl From<TeamRow> for TeamAggr {
//     fn from(v: TeamRow) -> Self {
//         TeamAggr {
//             id: v.f_id,
//             name: v.f_name,
//             description: v.f_description.unwrap_or_default(),
//             avatar_key: v.f_avatar_key,
//             avatar_uploaded: v.f_avatar_uploaded,
//             avatar_version: v.f_avatar_version,
//             workset_next_index: v.f_workset_next_index,
//             created_at: v.f_created_at,
//             updated_at: v.f_updated_at,
//         }
//     }
// }
