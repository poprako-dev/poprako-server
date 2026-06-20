// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// use diesel::prelude::*;
// use time::OffsetDateTime;
// 
// use crate::domain::model::aggr::workset::WorksetAggr;
// use crate::infra::query::schema;
// 
// // ── Queryable / Selectable ─────────────────────────────────────────────────
// 
// #[derive(Queryable, Selectable)]
// #[diesel(table_name = schema::t_workset)]
// pub struct WorksetRow {
//     pub f_id: String,
//     pub f_team_id: String,
//     pub f_index: i32,
//     pub f_name: String,
//     pub f_description: Option<String>,
//     pub f_comic_count: i32,
//     pub f_comic_next_index: i32,
//     pub f_created_at: OffsetDateTime,
//     pub f_updated_at: OffsetDateTime,
// }
// 
// // ── Insertable ─────────────────────────────────────────────────────────────
// 
// #[derive(Insertable)]
// #[diesel(table_name = schema::t_workset)]
// pub struct WorksetEntry<'a> {
//     pub f_id: &'a str,
//     pub f_team_id: &'a str,
//     pub f_index: i32,
//     pub f_name: &'a str,
//     pub f_description: Option<&'a str>,
//     pub f_created_at: OffsetDateTime,
//     pub f_updated_at: OffsetDateTime,
// }
// 
// // ── Changeset (AsChangeset) ────────────────────────────────────────────────
// 
// /// Changeset for updating workset fields via partial updates.
// ///
// /// Only `Some` fields are included in the generated `SET` clause;
// /// `None` fields are omitted.
// #[derive(AsChangeset)]
// #[diesel(table_name = schema::t_workset)]
// pub struct WorksetAspect<'a> {
//     pub f_name: Option<&'a str>,
//     pub f_description: Option<Option<&'a str>>,
//     pub f_comic_count: Option<i32>,
//     pub f_comic_next_index: Option<i32>,
//     pub f_updated_at: OffsetDateTime,
// }
// 
// impl<'a> WorksetAspect<'a> {
//     /// Creates a new changeset with all optional fields set to `None`.
//     pub fn new(updated_at: OffsetDateTime) -> Self {
//         Self {
//             f_name: None,
//             f_description: None,
//             f_comic_count: None,
//             f_comic_next_index: None,
//             f_updated_at: updated_at,
//         }
//     }
// 
//     pub fn name(mut self, val: &'a str) -> Self {
//         self.f_name = Some(val);
//         self
//     }
// 
//     /// Sets the description column.  Pass `None` to clear the column;
//     /// pass `Some(val)` to set it.
//     pub fn description(mut self, val: Option<&'a str>) -> Self {
//         self.f_description = Some(val);
//         self
//     }
// 
//     pub fn comic_count(mut self, val: i32) -> Self {
//         self.f_comic_count = Some(val);
//         self
//     }
// 
//     #[allow(dead_code)]
//     pub fn comic_next_index(mut self, val: i32) -> Self {
//         self.f_comic_next_index = Some(val);
//         self
//     }
// }
// 
// // ── Conversions ────────────────────────────────────────────────────────────
// 
// impl From<WorksetRow> for WorksetAggr {
//     fn from(v: WorksetRow) -> Self {
//         WorksetAggr {
//             id: v.f_id,
//             team_id: v.f_team_id,
//             team: None,
//             index: v.f_index,
//             name: v.f_name,
//             description: v.f_description,
//             comic_count: v.f_comic_count,
//             comic_next_index: v.f_comic_next_index,
//             created_at: v.f_created_at,
//             updated_at: v.f_updated_at,
//         }
//     }
// }
