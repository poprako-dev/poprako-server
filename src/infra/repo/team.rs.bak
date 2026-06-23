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
// use crate::domain::model::aggr::team::{TeamAggr, TeamAvatarReservation, TeamForm, TeamInfoUpdate};
// use crate::domain::repo_legacy::team::{TeamRepo, TeamRepoTransactional};
// use crate::domain::result::{DomainError, DomainResult};
// use crate::infra::repo::entity::team::{TeamAspect, TeamEntry, TeamRow};
// use crate::infra::repo::schema::t_team::dsl::*;
// use crate::infra::repo::{RdbQuery, RdbRepoTransactional};
// use crate::submit_query;
// 
// // ── Free functions ─────────────────────────────────────────────────────────
// 
// #[instrument(err, skip(conn), level = Level::DEBUG)]
// pub async fn get_by_id(conn: &mut AsyncPgConnection, id: &str) -> DomainResult<TeamAggr> {
//     let row: TeamRow = t_team
//         .filter(f_id.eq(&id))
//         .select(TeamRow::as_select())
//         .first(conn)
//         .await
//         .optional()?
//         .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))?;
// 
//     Ok(row.into())
// }
// 
// #[instrument(err, skip(conn), level = Level::DEBUG)]
// pub async fn list(conn: &mut AsyncPgConnection, page: Page) -> DomainResult<Vec<TeamAggr>> {
//     let rows: Vec<TeamRow> = t_team
//         .order(f_created_at.desc())
//         .offset(page.offset as i64)
//         .limit(page.limit as i64)
//         .select(TeamRow::as_select())
//         .load(conn)
//         .await?;
// 
//     let result: Vec<TeamAggr> = rows.into_iter().map(|r| r.into()).collect();
// 
//     Ok(result)
// }
// 
// #[instrument(err, skip(conn, form), level = Level::DEBUG)]
// pub async fn create(conn: &mut AsyncPgConnection, form: &TeamForm) -> DomainResult<TeamAggr> {
//     let now = OffsetDateTime::now_utc();
// 
//     let entry = TeamEntry {
//         f_id: &form.id,
//         f_name: &form.name,
//         f_description: &form.description,
//         f_workset_next_index: 0,
//         f_created_at: now,
//         f_updated_at: now,
//     };
// 
//     diesel::insert_into(t_team)
//         .values(&entry)
//         .execute(conn)
//         .await?;
// 
//     let row: TeamRow = t_team
//         .filter(f_id.eq(&entry.f_id))
//         .select(TeamRow::as_select())
//         .first(conn)
//         .await?;
// 
//     Ok(row.into())
// }
// 
// #[instrument(err, skip(conn, update), level = Level::DEBUG)]
// pub async fn update(conn: &mut AsyncPgConnection, update: &TeamInfoUpdate) -> DomainResult<()> {
//     let now = OffsetDateTime::now_utc();
// 
//     let changes = TeamAspect::new(now)
//         .name(&update.name)
//         .description(&update.description);
// 
//     let affected = diesel::update(t_team.filter(f_id.eq(&update.id)))
//         .set(&changes)
//         .execute(conn)
//         .await?;
// 
//     if affected == 0 {
//         return Err(DomainError::expected_argument(trl("error-team-not-found")));
//     }
// 
//     Ok(())
// }
// 
// #[instrument(err, skip(conn), level = Level::DEBUG)]
// pub async fn get_by_id_ex(conn: &mut AsyncPgConnection, id: &str) -> DomainResult<TeamAggr> {
//     let row: TeamRow = t_team
//         .filter(f_id.eq(&id))
//         .select(TeamRow::as_select())
//         .for_update()
//         .first(conn)
//         .await
//         .optional()?
//         .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))?;
// 
//     Ok(row.into())
// }
// 
// #[instrument(err, skip(conn), level = Level::DEBUG)]
// pub async fn reserve_avatar(
//     conn: &mut AsyncPgConnection,
//     id: &str,
//     file_extension: &str,
// ) -> DomainResult<TeamAvatarReservation> {
//     let team = get_by_id_ex(conn, id).await?;
//     let now = OffsetDateTime::now_utc();
//     let avatar_version = team.avatar_version + 1;
//     let object_key = TeamAggr::generate_avatar_key(id, avatar_version, file_extension);
//     let previous_object_key = team.avatar_key.clone();
// 
//     let changes = TeamAspect::new(now)
//         .avatar_key(&object_key)
//         .avatar_uploaded(false)
//         .avatar_version(avatar_version);
// 
//     let affected = diesel::update(t_team.filter(f_id.eq(id)))
//         .set(&changes)
//         .execute(conn)
//         .await?;
// 
//     if affected == 0 {
//         return Err(DomainError::expected_argument(trl("error-team-not-found")));
//     }
// 
//     Ok(TeamAvatarReservation {
//         object_key,
//         previous_object_key,
//         avatar_version,
//     })
// }
// 
// #[instrument(err, skip(conn), level = Level::DEBUG)]
// pub async fn mark_avatar_uploaded(
//     conn: &mut AsyncPgConnection,
//     id: &str,
//     avatar_version: i64,
// ) -> DomainResult<()> {
//     let team = get_by_id(conn, id).await?;
//     if team.avatar_version != avatar_version {
//         return Err(DomainError::expected_argument(trl(
//             "error-stale-avatar-upload",
//         )));
//     }
// 
//     if team.avatar_uploaded {
//         return Ok(());
//     }
// 
//     let now = OffsetDateTime::now_utc();
// 
//     let changes = TeamAspect::new(now).avatar_uploaded(true);
// 
//     let affected = diesel::update(t_team.filter(f_id.eq(id)))
//         .set(&changes)
//         .execute(conn)
//         .await?;
// 
//     if affected == 0 {
//         return Err(DomainError::expected_argument(trl("error-team-not-found")));
//     }
// 
//     Ok(())
// }
// 
// #[instrument(err, skip(conn), level = Level::DEBUG)]
// pub async fn delete(conn: &mut AsyncPgConnection, id: &str) -> DomainResult<()> {
//     let affected = diesel::delete(t_team.filter(f_id.eq(id)))
//         .execute(conn)
//         .await?;
// 
//     if affected == 0 {
//         return Err(DomainError::expected_argument(trl("error-team-not-found")));
//     }
// 
//     Ok(())
// }
// 
// #[instrument(err, skip(conn), level = Level::DEBUG)]
// pub async fn increment_workset_next_index(
//     conn: &mut AsyncPgConnection,
//     id: &str,
// ) -> DomainResult<i32> {
//     let affected = diesel::update(t_team.filter(f_id.eq(id)))
//         .set(f_workset_next_index.eq(f_workset_next_index + 1))
//         .execute(conn)
//         .await?;
// 
//     if affected == 0 {
//         return Err(DomainError::expected_argument(trl("error-team-not-found")));
//     }
// 
//     let new_value: i32 = t_team
//         .filter(f_id.eq(id))
//         .select(f_workset_next_index)
//         .first(conn)
//         .await?;
// 
//     // The column now holds the incremented value; subtract 1 to get the allocated index.
//     Ok(new_value - 1)
// }
// 
// // ── impls ──────────────────────────────────────────────────────────────────
// 
// #[async_trait]
// impl TeamRepo for RdbQuery {
//     #[instrument(err, skip(self), level = Level::DEBUG)]
//     async fn get_by_id(&self, id: &str) -> DomainResult<TeamAggr> {
//         submit_query!(self.pool, get_by_id, id)
//     }
// 
//     #[instrument(err, skip(self), level = Level::DEBUG)]
//     async fn list(&self, page: Page) -> DomainResult<Vec<TeamAggr>> {
//         submit_query!(self.pool, list, page)
//     }
// 
//     #[instrument(err, skip(self, form), level = Level::DEBUG)]
//     async fn create(&self, form: &TeamForm) -> DomainResult<TeamAggr> {
//         submit_query!(self.pool, create, form)
//     }
// 
//     #[instrument(err, skip(self, params), level = Level::DEBUG)]
//     async fn update_info(&self, params: &TeamInfoUpdate) -> DomainResult<()> {
//         submit_query!(self.pool, update, params)
//     }
// 
//     #[instrument(err, skip(self), level = Level::DEBUG)]
//     async fn mark_avatar_uploaded(&self, id: &str, avatar_version: i64) -> DomainResult<()> {
//         submit_query!(self.pool, mark_avatar_uploaded, id, avatar_version)
//     }
// }
// 
// #[async_trait]
// impl<'c> TeamRepoTransactional for RdbRepoTransactional<'c> {
//     async fn increment_workset_next_index(&mut self, id: &str) -> DomainResult<i32> {
//         increment_workset_next_index(self.conn, id).await
//     }
// 
//     async fn get_by_id_excluded(&mut self, id: &str) -> DomainResult<TeamAggr> {
//         get_by_id_ex(self.conn, id).await
//     }
// 
//     async fn reserve_avatar(
//         &mut self,
//         id: &str,
//         file_extension: &str,
//     ) -> DomainResult<TeamAvatarReservation> {
//         reserve_avatar(self.conn, id, file_extension).await
//     }
// 
//     async fn delete(&mut self, id: &str) -> DomainResult<()> {
//         delete(self.conn, id).await
//     }
// 
//     async fn mark_avatar_uploaded(&mut self, id: &str, avatar_version: i64) -> DomainResult<()> {
//         mark_avatar_uploaded(self.conn, id, avatar_version).await
//     }
// }
