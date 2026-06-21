// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// use async_trait::async_trait;
// use diesel::prelude::*;
// use diesel_async::{AsyncPgConnection, RunQueryDsl};
// use time::OffsetDateTime;
// use tracing::{Level, instrument};
// 
// use poprako_util::i18n::trl;
// use poprako_util::page::Page;
// 
// use crate::domain::model::aggr::workset::{WorksetAggr, WorksetForm, WorksetUpdate};
// use crate::domain::repo_legacy::workset::{WorksetRepo, WorksetRepoTransactional};
// use crate::domain::result::{DomainError, DomainResult};
// use crate::infra::repo::entity::workset::{WorksetAspect, WorksetEntry, WorksetRow};
// use crate::infra::repo::schema::t_workset::dsl::*;
// use crate::infra::repo::{RdbQuery, RdbRepoTransactional};
// use crate::submit_query;
// 
// // ── Free functions ─────────────────────────────────────────────────────────
// 
// #[instrument(err, skip(conn), level = Level::DEBUG)]
// pub async fn get_by_id(
//     conn: &mut AsyncPgConnection,
//     workset_id: &str,
// ) -> DomainResult<WorksetAggr> {
//     let row: WorksetRow = t_workset
//         .filter(f_id.eq(&workset_id))
//         .select(WorksetRow::as_select())
//         .first(conn)
//         .await
//         .optional()?
//         .ok_or_else(|| DomainError::expected_argument(trl("error-workset-not-found")))?;
// 
//     Ok(row.into())
// }
// 
// /// Joins with `t_team` to preload the owning team on each workset row.
// #[instrument(err, skip(conn), level = Level::DEBUG)]
// pub async fn list(
//     conn: &mut AsyncPgConnection,
//     team_id: &str,
//     page: Page,
// ) -> DomainResult<Vec<WorksetAggr>> {
//     use crate::infra::repo::entity::team::TeamRow;
//     use crate::infra::repo::schema::t_team;
// 
//     let rows: Vec<(WorksetRow, TeamRow)> = t_workset
//         .inner_join(t_team::table)
//         .filter(f_team_id.eq(team_id))
//         .order(f_index.asc())
//         .offset(page.offset as i64)
//         .limit(page.limit as i64)
//         .select((WorksetRow::as_select(), TeamRow::as_select()))
//         .load(conn)
//         .await?;
// 
//     let result: Vec<WorksetAggr> = rows
//         .into_iter()
//         .map(|(workset_row, team_row)| {
//             let mut aggr: WorksetAggr = workset_row.into();
//             aggr.team = Some(team_row.into());
//             aggr
//         })
//         .collect();
// 
//     Ok(result)
// }
// 
// #[instrument(err, skip(conn), level = Level::DEBUG)]
// pub async fn count(conn: &mut AsyncPgConnection, team_id: &str) -> DomainResult<i64> {
//     let total: i64 = t_workset
//         .filter(f_team_id.eq(team_id))
//         .count()
//         .get_result(conn)
//         .await?;
// 
//     Ok(total)
// }
// 
// #[instrument(err, skip(conn, form), level = Level::DEBUG)]
// pub async fn create(conn: &mut AsyncPgConnection, form: &WorksetForm) -> DomainResult<WorksetAggr> {
//     let now = OffsetDateTime::now_utc();
// 
//     let entry = WorksetEntry {
//         f_id: &form.id,
//         f_team_id: &form.team_id,
//         f_index: form.index,
//         f_name: &form.name,
//         f_description: form.description.as_deref(),
//         f_created_at: now,
//         f_updated_at: now,
//     };
// 
//     diesel::insert_into(t_workset)
//         .values(&entry)
//         .execute(conn)
//         .await?;
// 
//     let row: WorksetRow = t_workset
//         .filter(f_id.eq(&entry.f_id))
//         .select(WorksetRow::as_select())
//         .first(conn)
//         .await?;
// 
//     Ok(row.into())
// }
// 
// #[instrument(err, skip(conn, update), level = Level::DEBUG)]
// pub async fn update(conn: &mut AsyncPgConnection, update: &WorksetUpdate) -> DomainResult<()> {
//     let now = OffsetDateTime::now_utc();
// 
//     let changes = WorksetAspect::new(now)
//         .name(&update.name)
//         .description(update.description.as_deref());
// 
//     let affected = diesel::update(t_workset.filter(f_id.eq(&update.id)))
//         .set(&changes)
//         .execute(conn)
//         .await?;
// 
//     if affected == 0 {
//         return Err(DomainError::expected_argument(trl(
//             "error-workset-not-found",
//         )));
//     }
// 
//     Ok(())
// }
// 
// #[instrument(err, skip(conn), level = Level::DEBUG)]
// pub async fn update_comic_count(
//     conn: &mut AsyncPgConnection,
//     workset_id: &str,
//     delta: i32,
// ) -> DomainResult<()> {
//     let now = OffsetDateTime::now_utc();
// 
//     let current: i32 = t_workset
//         .filter(f_id.eq(workset_id))
//         .select(f_comic_count)
//         .first(conn)
//         .await
//         .optional()?
//         .ok_or_else(|| DomainError::expected_argument(trl("error-workset-not-found")))?;
// 
//     let new_value = std::cmp::max(current + delta, 0);
// 
//     diesel::update(t_workset.filter(f_id.eq(workset_id)))
//         .set((f_comic_count.eq(new_value), f_updated_at.eq(now)))
//         .execute(conn)
//         .await?;
// 
//     Ok(())
// }
// 
// #[instrument(err, skip(conn), level = Level::DEBUG)]
// pub async fn increment_comic_next_index(
//     conn: &mut AsyncPgConnection,
//     workset_id: &str,
// ) -> DomainResult<i32> {
//     let affected = diesel::update(t_workset.filter(f_id.eq(workset_id)))
//         .set(f_comic_next_index.eq(f_comic_next_index + 1))
//         .execute(conn)
//         .await?;
// 
//     if affected == 0 {
//         return Err(DomainError::expected_argument(trl(
//             "error-workset-not-found",
//         )));
//     }
// 
//     let new_value: i32 = t_workset
//         .filter(f_id.eq(workset_id))
//         .select(f_comic_next_index)
//         .first(conn)
//         .await?;
// 
//     // The column now holds the incremented value; subtract 1 to get the allocated index.
//     Ok(new_value - 1)
// }
// 
// #[instrument(err, skip(conn), level = Level::DEBUG)]
// pub async fn delete(conn: &mut AsyncPgConnection, workset_id: &str) -> DomainResult<()> {
//     let affected = diesel::delete(t_workset.filter(f_id.eq(workset_id)))
//         .execute(conn)
//         .await?;
// 
//     if affected == 0 {
//         return Err(DomainError::expected_argument(trl(
//             "error-workset-not-found",
//         )));
//     }
// 
//     Ok(())
// }
// 
// #[instrument(err, skip(conn), level = Level::DEBUG)]
// pub async fn list_by_team_id_excluded(
//     conn: &mut AsyncPgConnection,
//     team_id: &str,
// ) -> DomainResult<Vec<WorksetAggr>> {
//     let rows: Vec<WorksetRow> = t_workset
//         .filter(f_team_id.eq(team_id))
//         .for_update()
//         .select(WorksetRow::as_select())
//         .load(conn)
//         .await?;
// 
//     Ok(rows.into_iter().map(|r| r.into()).collect())
// }
// 
// // ── impls ──────────────────────────────────────────────────────────────────
// 
// #[async_trait]
// impl WorksetRepo for RdbQuery {
//     #[instrument(err, skip(self), level = Level::DEBUG)]
//     async fn get_by_id(&self, id: &str) -> DomainResult<WorksetAggr> {
//         submit_query!(self.pool, get_by_id, id)
//     }
// 
//     #[instrument(err, skip(self), level = Level::DEBUG)]
//     async fn list(&self, team_id: &str, page: Page) -> DomainResult<Vec<WorksetAggr>> {
//         submit_query!(self.pool, list, team_id, page)
//     }
// 
//     #[instrument(err, skip(self), level = Level::DEBUG)]
//     async fn count(&self, team_id: &str) -> DomainResult<i64> {
//         submit_query!(self.pool, count, team_id)
//     }
// 
//     #[instrument(err, skip(self, params), level = Level::DEBUG)]
//     async fn update(&self, params: &WorksetUpdate) -> DomainResult<()> {
//         submit_query!(self.pool, update, params)
//     }
// }
// 
// #[async_trait]
// impl<'c> WorksetRepoTransactional for RdbRepoTransactional<'c> {
//     async fn create(&mut self, form: &WorksetForm) -> DomainResult<WorksetAggr> {
//         create(self.conn, form).await
//     }
// 
//     async fn update_comic_count(&mut self, id: &str, delta: i32) -> DomainResult<()> {
//         update_comic_count(self.conn, id, delta).await
//     }
// 
//     async fn increment_comic_next_index(&mut self, id: &str) -> DomainResult<i32> {
//         increment_comic_next_index(self.conn, id).await
//     }
// 
//     async fn delete(&mut self, id: &str) -> DomainResult<()> {
//         delete(self.conn, id).await
//     }
// 
//     async fn list_by_team_id_excluded(&mut self, team_id: &str) -> DomainResult<Vec<WorksetAggr>> {
//         list_by_team_id_excluded(self.conn, team_id).await
//     }
// }
