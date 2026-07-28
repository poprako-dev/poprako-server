//! RDB-backed atomic comic archive repository.

use std::collections::HashMap;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::comic_archive::{
    ComicArchiveChapterSnapshot, ComicArchivePageSnapshot, ComicArchiveSnapshot,
};
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::UnitInfo;
use crate::model::read::proj::user::UserInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::write::comic_archive::ComicArchiveEntry;
use crate::part::repo::oper::comic_archive::{
    CommitComicArchive, GetComicArchiveSnapshotExcluded,
    ListComicArchivePayloads,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::assignment::AssignmentRow;
use crate::part_impl::repo::rdb_impl::entity::chapter::ChapterRow;
use crate::part_impl::repo::rdb_impl::entity::comic::ComicRow;
use crate::part_impl::repo::rdb_impl::entity::comic_archive::ComicArchiveRow;
use crate::part_impl::repo::rdb_impl::entity::page::PageRow;
use crate::part_impl::repo::rdb_impl::entity::unit::UnitRow;
use crate::part_impl::repo::rdb_impl::entity::user::UserRow;
use crate::part_impl::repo::rdb_impl::entity::workset::WorksetRow;
use crate::part_impl::repo::rdb_impl::schema::t_assignment::dsl::{
    f_chapter_id as assignment_chapter_id, t_assignment,
};
use crate::part_impl::repo::rdb_impl::schema::t_assignment_invitation::dsl::{
    f_chapter_id as invitation_chapter_id, f_id as invitation_id,
    t_assignment_invitation,
};
use crate::part_impl::repo::rdb_impl::schema::t_chapter::dsl::{
    f_comic_id as chapter_comic_id, f_id as chapter_id, t_chapter,
};
use crate::part_impl::repo::rdb_impl::schema::t_comic::dsl::{
    f_id as comic_id, t_comic,
};
use crate::part_impl::repo::rdb_impl::schema::t_comic_archive;
use crate::part_impl::repo::rdb_impl::schema::t_page::dsl::{
    f_chapter_id as page_chapter_id, f_id as page_id, f_index as page_index,
    t_page,
};
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{
    f_page_id as unit_page_id, t_unit,
};
use crate::part_impl::repo::rdb_impl::schema::t_user::dsl::{
    f_id as user_id, t_user,
};
use crate::part_impl::repo::rdb_impl::schema::t_workset::dsl::{
    f_id as workset_id, t_workset,
};
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{BaseError, BaseRest, accept};
use crate::value::comic_archive::ComicArchiveMonth;

/// Comic archive RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

// Standardize chain-corruption failures for unit graph validation.
fn corrupt_unit_chain_err() -> BaseError {
    BaseError::Unrecoverable {
        message: "persisted Unit chain is corrupt".to_string(),
    }
}

// Reorder chained unit infos by next_id links and return only visible units.
fn order_unit_infos(unit_infos: Vec<UnitInfo>) -> BaseRest<Vec<UnitInfo>> {
    //
    if unit_infos.is_empty() {
        return accept(Vec::new());
    }

    let mut infos_by_id = unit_infos
        .into_iter()
        .map(|unit_info| (unit_info.id.clone(), unit_info))
        .collect::<HashMap<_, _>>();

    let mut predecessor_counts = infos_by_id
        .keys()
        .map(|id| (id.clone(), 0_usize))
        .collect::<HashMap<_, _>>();

    for unit_info in infos_by_id.values() {
        //
        let Some(next_id) = unit_info.next_id.as_ref() else {
            continue;
        };

        if next_id == &unit_info.id {
            return Err(corrupt_unit_chain_err());
        }

        let Some(predecessor_count) = predecessor_counts.get_mut(next_id)
        else {
            return Err(corrupt_unit_chain_err());
        };

        *predecessor_count += 1;

        if *predecessor_count > 1 {
            return Err(corrupt_unit_chain_err());
        }
    }

    let head_ids = predecessor_counts
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(id.as_str()))
        .collect::<Vec<_>>();

    let [head_id] = head_ids.as_slice() else {
        return Err(corrupt_unit_chain_err());
    };

    let mut current_id = Some((*head_id).to_string());

    let mut visible_infos = Vec::with_capacity(infos_by_id.len());

    while let Some(id) = current_id {
        //
        let Some(unit_info) = infos_by_id.remove(&id) else {
            return Err(corrupt_unit_chain_err());
        };

        current_id = unit_info.next_id.clone();

        if unit_info.hidden_at.is_none() {
            visible_infos.push(unit_info);
        }
    }

    if !infos_by_id.is_empty() {
        return Err(corrupt_unit_chain_err());
    }

    accept(visible_infos)
}

#[instrument(level = "info", err(Debug), skip_all)]
// Load archive payloads by month window and return timestamped serialized blobs.
async fn list_payloads(
    conn: &mut RdbConn,
    team_id: &str,
    months: &[ComicArchiveMonth],
) -> BaseRest<Vec<(OffsetDateTime, String)>> {
    //
    #[derive(Queryable)]
    struct ArchivePayloadRow {
        //
        created_at: OffsetDateTime,
        // Serialized payload snapshot JSON for a retention slot.
        payload: String,
    }

    use crate::part_impl::repo::rdb_impl::schema::t_comic_archive::dsl::{
        f_archived_payload, f_created_at, f_team_id, t_comic_archive,
    };

    let Some(first_month) = months.first() else {
        return accept(Vec::new());
    };

    let Some(last_month) = months.last() else {
        return accept(Vec::new());
    };

    let query = t_comic_archive
        .filter(f_team_id.eq(team_id))
        .filter(f_created_at.ge(first_month.start))
        .filter(f_created_at.lt(last_month.end))
        .select((f_created_at, f_archived_payload))
        .into_boxed();

    let rows: Vec<ArchivePayloadRow> = query
        .order_by(f_created_at.asc())
        .load(conn)
        .await
        .map_err(diesel)?;

    accept(
        rows.into_iter()
            .filter(|row| {
                months.iter().any(|month| {
                    row.created_at >= month.start && row.created_at < month.end
                })
            })
            .map(|row| (row.created_at, row.payload))
            .collect(),
    )
}

/// Lock every active descendant needed by an archive transaction.
#[instrument(level = "info", err(Debug), skip_all)]
// Build a full snapshot of all descendants and lock them for commit safety.
async fn get_snapshot_excluded(
    conn: &mut RdbConn,
    source_comic_id: &str,
) -> BaseRest<ComicArchiveSnapshot> {
    //
    let comic_row: ComicRow = t_comic
        .filter(comic_id.eq(source_comic_id))
        .select(ComicRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-comic-not-found"))?;

    let comic_info: ComicInfo = comic_row.try_into()?;

    let workset_row: WorksetRow = t_workset
        .filter(workset_id.eq(&comic_info.workset_id))
        .select(WorksetRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-workset-not-found"))?;

    let workset_info: WorksetInfo = workset_row.into();

    let chapter_rows: Vec<ChapterRow> = t_chapter
        .filter(chapter_comic_id.eq(&comic_info.id))
        .select(ChapterRow::as_select())
        .order_by(chapter_id.asc())
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    let chapter_infos: Vec<ChapterInfo> = chapter_rows
        .into_iter()
        .map(ChapterInfo::try_from)
        .collect::<BaseRest<Vec<_>>>()?;

    let source_chapter_ids = chapter_infos
        .iter()
        .map(|chapter_info| chapter_info.id.clone())
        .collect::<Vec<_>>();

    let _: Vec<String> = t_assignment_invitation
        .filter(invitation_chapter_id.eq_any(&source_chapter_ids))
        .select(invitation_id)
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    let assignment_rows: Vec<AssignmentRow> = t_assignment
        .filter(assignment_chapter_id.eq_any(&source_chapter_ids))
        .select(AssignmentRow::as_select())
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    let assignment_infos = assignment_rows
        .into_iter()
        .map(AssignmentInfo::try_from)
        .collect::<BaseRest<Vec<_>>>()?;

    let assigned_user_ids = assignment_infos
        .iter()
        .map(|assignment_info| assignment_info.user_id.clone())
        .collect::<Vec<_>>();

    let user_rows: Vec<UserRow> = t_user
        .filter(user_id.eq_any(&assigned_user_ids))
        .select(UserRow::as_select())
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    let user_infos = user_rows
        .into_iter()
        .map(|user_row| {
            //
            let user_info: UserInfo = user_row.try_into()?;

            Ok((user_info.id.clone(), user_info))
        })
        .collect::<BaseRest<HashMap<_, _>>>()?;

    let page_rows: Vec<PageRow> = t_page
        .filter(page_chapter_id.eq_any(&source_chapter_ids))
        .select(PageRow::as_select())
        .order_by((page_chapter_id.asc(), page_index.asc(), page_id.asc()))
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    let page_infos: Vec<PageInfo> = page_rows
        .into_iter()
        .map(TryInto::try_into)
        .collect::<BaseRest<_>>()?;

    let source_page_ids = page_infos
        .iter()
        .map(|page_info| page_info.id.clone())
        .collect::<Vec<_>>();

    let unit_rows: Vec<UnitRow> = t_unit
        .filter(unit_page_id.eq_any(&source_page_ids))
        .select(UnitRow::as_select())
        .order_by(unit_page_id.asc())
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    let unit_infos: Vec<UnitInfo> =
        unit_rows.into_iter().map(Into::into).collect::<Vec<_>>();

    let mut assignment_infos_by_chapter: HashMap<String, Vec<AssignmentInfo>> =
        HashMap::new();

    for mut assignment_info in assignment_infos {
        //
        assignment_info.user = Some(
            user_infos
                .get(&assignment_info.user_id)
                .cloned()
                .ok_or_else(|| expected("error-user-not-found"))?,
        );

        assignment_infos_by_chapter
            .entry(assignment_info.chapter_id.clone())
            .or_default()
            .push(assignment_info);
    }

    let mut unit_infos_by_page = HashMap::new();

    for unit_info in unit_infos {
        unit_infos_by_page
            .entry(unit_info.page_id.clone())
            .or_insert_with(Vec::new)
            .push(unit_info);
    }

    let mut page_snapshots_by_chapter = HashMap::new();

    for page_info in page_infos {
        //
        let unordered_unit_infos =
            unit_infos_by_page.remove(&page_info.id).unwrap_or_default();

        let mut unit_infos = order_unit_infos(unordered_unit_infos)?;

        unit_infos.retain(|unit_info| unit_info.hidden_at.is_none());

        page_snapshots_by_chapter
            .entry(page_info.chapter_id.clone())
            .or_insert_with(Vec::new)
            .push(ComicArchivePageSnapshot {
                page_info,
                unit_infos,
            });
    }

    let chapter_snapshots = chapter_infos
        .into_iter()
        .map(|chapter_info| {
            //
            let assignment_infos = assignment_infos_by_chapter
                .remove(&chapter_info.id)
                .unwrap_or_default();

            let page_snapshots = page_snapshots_by_chapter
                .remove(&chapter_info.id)
                .unwrap_or_default();

            ComicArchiveChapterSnapshot {
                chapter_info,
                assignment_infos,
                page_snapshots,
            }
        })
        .collect();

    accept(ComicArchiveSnapshot {
        comic_info,
        workset_info,
        chapter_snapshots,
    })
}

// Store archive payload and hard-delete source comic descendants.
#[instrument(level = "info", err(Debug), skip_all)]
async fn commit(
    conn: &mut RdbConn,
    comic_archive_entry: &ComicArchiveEntry,
) -> BaseRest<()> {
    //
    let comic_archive_row = ComicArchiveRow::from(&comic_archive_entry.record);

    diesel::insert_into(t_comic_archive::table)
        .values(&comic_archive_row)
        .execute(conn)
        .await
        .map_err(diesel)?;

    diesel::delete(t_assignment_invitation.filter(
        invitation_chapter_id.eq_any(&comic_archive_entry.source_chapter_ids),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(t_assignment.filter(
        assignment_chapter_id.eq_any(&comic_archive_entry.source_chapter_ids),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_unit
            .filter(unit_page_id.eq_any(&comic_archive_entry.source_page_ids)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(t_page.filter(
        page_chapter_id.eq_any(&comic_archive_entry.source_chapter_ids),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_chapter
            .filter(chapter_id.eq_any(&comic_archive_entry.source_chapter_ids)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_comic.filter(comic_id.eq(&comic_archive_entry.source_comic_id)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}

impl Step<GetComicArchiveSnapshotExcluded<'_>, RdbContext> for RdbRepo {
    // Use base errors for snapshot reads in comic archive transactions.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Resolve the snapshot while holding transaction locks.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetComicArchiveSnapshotExcluded<'_>,
    ) -> BaseRest<ComicArchiveSnapshot> {
        get_snapshot_excluded(context.conn(), oper.comic_id).await
    }
}

impl Run<ListComicArchivePayloads<'_>> for RdbRepo {
    // Use base errors for payload-list operations.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Route to shared payload query with team-month filters.
    async fn run(
        &self,
        oper: &ListComicArchivePayloads<'_>,
    ) -> BaseRest<Vec<(OffsetDateTime, String)>> {
        submit_query!(self.core, list_payloads, oper.team_id, oper.months)
    }
}

impl Step<CommitComicArchive<'_>, RdbContext> for RdbRepo {
    // Use base errors for commit operations.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Persist archive entry and delete source entities in a single transaction.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CommitComicArchive<'_>,
    ) -> BaseRest<()> {
        commit(context.conn(), oper.entry).await
    }
}
