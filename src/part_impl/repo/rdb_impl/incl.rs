//! Generic batch-include abstraction — reusable across all entity repos.
//!
//! ## Architecture
//!
//! Two-tier trait design:
//!
//! 1. [`BatchByIds`] — per-table, defined once. Knows how to `SELECT * FROM t_xxx WHERE
//!    f_id IN (...)` and convert rows to domain infos. Shared across every entity that
//!    needs that table's data as an include.
//!
//! 2. [`Incl`] — per-include-variant, ~7 lines each. Declares which owner type, which
//!    related type, how to extract the FK, and how to set the loaded value. Delegates
//!    the actual query to its associated [`BatchByIds`].
//!
//! The generic [`populate`] function drives the whole pipeline: collect FKs → batch-load
//! via [`batch_load`] → populate every owner. Works on `&mut [Owner]` (list) or
//! `&mut Owner` (single item via `std::slice::from_mut`).

use std::collections::HashMap;

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::model::workset::WorksetInfo;
use crate::part_impl::shared::RdbConn;
use crate::part_impl::shared::result::diesel;
use crate::part_impl::repo::rdb_impl::entity::chapter::ChapterRow;
use crate::part_impl::repo::rdb_impl::entity::comic::ComicRow;
use crate::part_impl::repo::rdb_impl::entity::team::TeamRow;
use crate::part_impl::repo::rdb_impl::entity::user::UserRow;
use crate::part_impl::repo::rdb_impl::entity::workset::WorksetRow;
use crate::part_impl::repo::rdb_impl::schema;
use crate::result::RegularResult;

pub mod announcement;
pub mod assignment;
pub mod chapter;
pub mod comic;
pub mod comment;
pub mod member;
pub mod member_invitation;

// ── BatchByIds trait ────────────────────────────────────────────────────────

/// Per-table batch loader — implemented once per database table.
///
/// Shared across every entity repo that needs eager-loading of this table's data.
#[async_trait]
pub trait BatchByIds {
    /// The Diesel row type (Queryable + Selectable).
    type Row: Send;

    /// The domain info type produced from each row.
    type Info: Clone + Send;

    /// Execute `SELECT * FROM table WHERE f_id IN (...)`.
    async fn load(
        conn: &mut RdbConn,
        ids: Vec<&str>,
    ) -> RegularResult<Vec<Self::Row>>;

    /// Convert a row into its id key and domain info.
    fn into_entry(row: Self::Row) -> RegularResult<(String, Self::Info)>;
}

// ── Incl trait ──────────────────────────────────────────────────────────────

/// A single include variant — e.g. "load Comic's creator User".
///
/// Implementations are ~7-line unit structs. The associated [`BatchByIds`] handles
/// the SQL; this trait only declares FK extraction and field assignment.
#[async_trait]
pub trait Incl {
    /// The entity that owns the optional foreign key.
    type Owner;

    /// The related entity being loaded.
    type Related: Clone;

    /// Which [`BatchByIds`] to use for the database query.
    type Query: BatchByIds<Info = Self::Related>;

    /// Extract the foreign key from an owner. Returns `None` for chained
    /// includes whose prerequisite hasn't been loaded yet (e.g. Comic→Team
    /// when Workset hasn't been included).
    fn resolve_key(owner: &Self::Owner) -> Option<&str>;

    /// Set the loaded related entity on the owner.
    fn inject(owner: &mut Self::Owner, related: Option<Self::Related>);
}

// ── Generic engine ──────────────────────────────────────────────────────────

/// Batch-load related entities and populate every owner in `infos`.
///
/// This is the only include-driving function. Call it once per requested include
/// variant. Works on slices (list) or single items (via `from_mut`).
pub async fn populate<I: Incl>(
    conn: &mut RdbConn,
    infos: &mut [I::Owner],
) -> RegularResult<()> {
    let mut key_counts = HashMap::new();

    for owner in infos.iter() {
        let Some(key) = I::resolve_key(owner) else {
            continue;
        };
        *key_counts.entry(key.to_owned()).or_insert(0) += 1;
    }

    if key_counts.is_empty() {
        return Ok(());
    }

    let id_refs = key_counts.keys().map(String::as_str).collect::<Vec<_>>();

    let mut map = batch_load::<I::Query>(conn, id_refs).await?;
    for owner in infos.iter_mut() {
        let related = I::resolve_key(owner).and_then(|key| {
            take_loaded_related(&mut map, &mut key_counts, key)
        });

        I::inject(owner, related);
    }

    Ok(())
}

fn take_loaded_related<Related: Clone>(
    map: &mut HashMap<String, Related>,
    key_counts: &mut HashMap<String, usize>,
    key: &str,
) -> Option<Related> {
    let count = key_counts.get_mut(key)?;

    if *count <= 1 {
        key_counts.remove(key);
        return map.remove(key);
    }

    *count -= 1;

    map.get(key).cloned()
}

/// Execute a batch `SELECT … WHERE f_id IN (…)` via a [`BatchByIds`] impl.
async fn batch_load<B: BatchByIds>(
    conn: &mut RdbConn,
    ids: Vec<&str>,
) -> RegularResult<HashMap<String, B::Info>> {
    let rows = B::load(conn, ids).await?;

    let mut map = HashMap::new();
    for row in rows {
        let (id, info) = B::into_entry(row)?;
        map.insert(id, info);
    }

    Ok(map)
}

// ── Reusable BatchByIds impls (one per table) ───────────────────────────────

pub struct UserByIds;

#[async_trait]
impl BatchByIds for UserByIds {
    type Row = UserRow;
    type Info = UserInfo;

    async fn load(
        conn: &mut RdbConn,
        ids: Vec<&str>,
    ) -> RegularResult<Vec<UserRow>> {
        schema::t_user::table
            .filter(schema::t_user::f_id.eq_any(ids))
            .select(UserRow::as_select())
            .load(conn)
            .await
            .map_err(diesel)
    }

    fn into_entry(row: UserRow) -> RegularResult<(String, UserInfo)> {
        let id = row.f_id.clone();
        Ok((id, UserInfo::from(row)))
    }
}

pub struct TeamByIds;

#[async_trait]
impl BatchByIds for TeamByIds {
    type Row = TeamRow;
    type Info = TeamInfo;

    async fn load(
        conn: &mut RdbConn,
        ids: Vec<&str>,
    ) -> RegularResult<Vec<TeamRow>> {
        schema::t_team::table
            .filter(schema::t_team::f_id.eq_any(ids))
            .select(TeamRow::as_select())
            .load(conn)
            .await
            .map_err(diesel)
    }

    fn into_entry(row: TeamRow) -> RegularResult<(String, TeamInfo)> {
        let id = row.f_id.clone();
        Ok((id, TeamInfo::from(row)))
    }
}

pub struct WorksetByIds;

#[async_trait]
impl BatchByIds for WorksetByIds {
    type Row = WorksetRow;
    type Info = WorksetInfo;

    async fn load(
        conn: &mut RdbConn,
        ids: Vec<&str>,
    ) -> RegularResult<Vec<WorksetRow>> {
        schema::t_workset::table
            .filter(schema::t_workset::f_id.eq_any(ids))
            .select(WorksetRow::as_select())
            .load(conn)
            .await
            .map_err(diesel)
    }

    fn into_entry(row: WorksetRow) -> RegularResult<(String, WorksetInfo)> {
        let id = row.f_id.clone();
        Ok((id, WorksetInfo::from(row)))
    }
}

pub struct ComicByIds;

#[async_trait]
impl BatchByIds for ComicByIds {
    type Row = ComicRow;
    type Info = ComicInfo;

    async fn load(
        conn: &mut RdbConn,
        ids: Vec<&str>,
    ) -> RegularResult<Vec<ComicRow>> {
        schema::t_comic::table
            .filter(schema::t_comic::f_id.eq_any(ids))
            .select(ComicRow::as_select())
            .load(conn)
            .await
            .map_err(diesel)
    }

    fn into_entry(row: ComicRow) -> RegularResult<(String, ComicInfo)> {
        let id = row.f_id.clone();
        Ok((id, ComicInfo::from(row)))
    }
}

pub struct ChapterByIds;

#[async_trait]
impl BatchByIds for ChapterByIds {
    type Row = ChapterRow;
    type Info = ChapterInfo;

    async fn load(
        conn: &mut RdbConn,
        ids: Vec<&str>,
    ) -> RegularResult<Vec<ChapterRow>> {
        schema::t_chapter::table
            .filter(schema::t_chapter::f_id.eq_any(ids))
            .select(ChapterRow::as_select())
            .load(conn)
            .await
            .map_err(diesel)
    }

    fn into_entry(row: ChapterRow) -> RegularResult<(String, ChapterInfo)> {
        let id = row.f_id.clone();

        let chapter_info = ChapterInfo::try_from(row)?;

        Ok((id, chapter_info))
    }
}
