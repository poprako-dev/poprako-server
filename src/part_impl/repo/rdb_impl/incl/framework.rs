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
use std::future::Future;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tracing::instrument;

use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::model::workset::WorksetInfo;
use crate::part_impl::repo::rdb_impl::entity::chapter::ChapterRow;
use crate::part_impl::repo::rdb_impl::entity::comic::ComicRow;
use crate::part_impl::repo::rdb_impl::entity::team::TeamRow;
use crate::part_impl::repo::rdb_impl::entity::user::UserRow;
use crate::part_impl::repo::rdb_impl::entity::workset::WorksetRow;
use crate::part_impl::repo::rdb_impl::schema::{t_chapter, t_comic, t_team, t_user, t_workset};
use crate::part_impl::shared::RdbConn;
use crate::part_impl::shared::result::diesel;
use crate::result::{BaseResult, accept};

// ── BatchByIds trait ────────────────────────────────────────────────────────

/// Per-table batch loader — implemented once per database table.
///
/// Shared across every entity repo that needs eager-loading of this table's data.
pub trait BatchByIds {
    /// The Diesel row type (Queryable + Selectable).
    type Row: Send;

    /// The domain info type produced from each row.
    type Info: Clone + Send;

    /// Execute `SELECT * FROM table WHERE f_id IN (...)`.
    fn load(
        conn: &mut RdbConn,
        ids: Vec<&str>,
    ) -> impl Future<Output = BaseResult<Vec<Self::Row>>> + Send;

    /// Convert a row into its id key and domain info.
    fn into_entry(row: Self::Row) -> BaseResult<(String, Self::Info)>;
}

// ── Incl trait ──────────────────────────────────────────────────────────────

/// A single include variant — e.g. "load Comic's creator User".
///
/// Implementations are ~7-line unit structs. The associated [`BatchByIds`] handles
/// the SQL; this trait only declares FK extraction and field assignment.
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
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn populate<I: Incl>(
    conn: &mut RdbConn,
    infos: &mut [I::Owner],
) -> BaseResult<()> {
    //
    let mut key_counts = HashMap::new();

    for owner in infos.iter() {
        //
        let Some(key) = I::resolve_key(owner) else {
            continue;
        };

        *key_counts.entry(key.to_owned()).or_insert(0) += 1;
    }

    if key_counts.is_empty() {
        return accept(());
    }

    let id_refs = key_counts.keys().map(String::as_str).collect::<Vec<_>>();

    let mut map = batch_load::<I::Query>(conn, id_refs).await?;

    for owner in infos.iter_mut() {
        //
        let related = I::resolve_key(owner).and_then(|key| {
            take_loaded_related(&mut map, &mut key_counts, key)
        });

        I::inject(owner, related);
    }

    accept(())
}

/// Decrements a reference count and takes ownership of a loaded related entity
/// when its last reference is consumed, avoiding a clone for shared entries.
fn take_loaded_related<Related: Clone>(
    map: &mut HashMap<String, Related>,
    key_counts: &mut HashMap<String, usize>,
    key: &str,
) -> Option<Related> {
    //
    let count = key_counts.get_mut(key)?;

    if *count <= 1 {
        //
        key_counts.remove(key);

        return map.remove(key);
    }

    *count -= 1;

    map.get(key).cloned()
}

/// Execute a batch `SELECT … WHERE f_id IN (…)` via a [`BatchByIds`] impl.
#[instrument(level = "info", err(Debug), skip_all)]
async fn batch_load<B: BatchByIds>(
    conn: &mut RdbConn,
    ids: Vec<&str>,
) -> BaseResult<HashMap<String, B::Info>> {
    //
    let rows = B::load(conn, ids).await?;

    let mut map = HashMap::new();

    for row in rows {
        //
        let (id, info) = B::into_entry(row)?;

        map.insert(id, info);
    }

    accept(map)
}

// ── Reusable BatchByIds impls (one per table) ───────────────────────────────

preload_by_ids! {
    UserByIds {
        row: UserRow,
        info: UserInfo,
        table: t_user,
        convert: TryFrom,
    }
    TeamByIds {
        row: TeamRow,
        info: TeamInfo,
        table: t_team,
        convert: TryFrom,
    }
    WorksetByIds {
        row: WorksetRow,
        info: WorksetInfo,
        table: t_workset,
        convert: From,
    }
    ComicByIds {
        row: ComicRow,
        info: ComicInfo,
        table: t_comic,
        convert: TryFrom,
    }
    ChapterByIds {
        row: ChapterRow,
        info: ChapterInfo,
        table: t_chapter,
        convert: TryFrom,
    }
}
