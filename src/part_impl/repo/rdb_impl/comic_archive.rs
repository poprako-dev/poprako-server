//! RDB-backed atomic comic archive repository.

use std::collections::HashMap;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use crate::model::assignment::AssignmentInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::comic_archive::{ComicArchiveChapterSnapshot, ComicArchiveEntry, ComicArchivePageSnapshot, ComicArchiveSnapshot};
use crate::model::page::PageInfo;
use crate::model::unit::UnitInfo;
use crate::model::user::UserInfo;
use crate::model::workset::WorksetInfo;
use crate::part::repo::oper::comic_archive::{CommitComicArchive, GetComicArchiveSnapshotExcluded, ListComicArchivePayloads};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::assignment::AssignmentRow;
use crate::part_impl::repo::rdb_impl::entity::chapter::ChapterRow;
use crate::part_impl::repo::rdb_impl::entity::comic::ComicRow;
use crate::part_impl::repo::rdb_impl::entity::comic_archive::ComicArchiveRow;
use crate::part_impl::repo::rdb_impl::entity::page::PageRow;
use crate::part_impl::repo::rdb_impl::entity::unit::UnitRow;
use crate::part_impl::repo::rdb_impl::entity::user::UserRow;
use crate::part_impl::repo::rdb_impl::entity::workset::WorksetRow;
use crate::part_impl::repo::rdb_impl::schema::t_assignment::dsl::{f_chapter_id as assignment_chapter_id, t_assignment};
use crate::part_impl::repo::rdb_impl::schema::t_assignment_invitation::dsl::{f_chapter_id as invitation_chapter_id, f_id as invitation_id, t_assignment_invitation};
use crate::part_impl::repo::rdb_impl::schema::t_chapter::dsl::{f_comic_id as chapter_comic_id, f_id as chapter_id, t_chapter};
use crate::part_impl::repo::rdb_impl::schema::t_comic::dsl::{f_id as comic_id, t_comic};
use crate::part_impl::repo::rdb_impl::schema::t_comic_archive;
use crate::part_impl::repo::rdb_impl::schema::t_page::dsl::{f_chapter_id as page_chapter_id, f_id as page_id, f_index as page_index, t_page};
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{f_index as unit_index, f_page_id as unit_page_id, t_unit};
use crate::part_impl::repo::rdb_impl::schema::t_user::dsl::{f_id as user_id, t_user};
use crate::part_impl::repo::rdb_impl::schema::t_workset::dsl::{f_id as workset_id, t_workset};
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{BaseError, BaseResult, accept};
use crate::value::comic_archive::ComicArchiveMonth;

#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

#[instrument(level = "info", err(Debug), skip_all)]
async fn list_payloads(
    conn: &mut RdbConn,
    team_id: &str,
    months: &[ComicArchiveMonth],
) -> BaseResult<Vec<(OffsetDateTime, String)>> {
    //
    #[derive(Queryable)]
    struct ArchivePayloadRow {
        created_at: OffsetDateTime,
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
async fn get_snapshot_excluded(
    conn: &mut RdbConn,
    source_comic_id: &str,
) -> BaseResult<ComicArchiveSnapshot> {
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
        .collect::<BaseResult<Vec<_>>>()?;

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
        .collect::<BaseResult<Vec<_>>>()?;

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
        .collect::<BaseResult<HashMap<_, _>>>()?;

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
        .collect::<BaseResult<_>>()?;

    let source_page_ids = page_infos
        .iter()
        .map(|page_info| page_info.id.clone())
        .collect::<Vec<_>>();

    let unit_rows: Vec<UnitRow> = t_unit
        .filter(unit_page_id.eq_any(&source_page_ids))
        .select(UnitRow::as_select())
        .order_by((unit_page_id.asc(), unit_index.asc()))
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
        let unit_infos =
            unit_infos_by_page.remove(&page_info.id).unwrap_or_default();

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

/// Insert one archive row and remove the active comic subtree without touching workset counters.
#[instrument(level = "info", err(Debug), skip_all)]
async fn commit(
    conn: &mut RdbConn,
    comic_archive_entry: &ComicArchiveEntry,
) -> BaseResult<()> {
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
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetComicArchiveSnapshotExcluded<'_>,
    ) -> BaseResult<ComicArchiveSnapshot> {
        get_snapshot_excluded(context.conn(), oper.comic_id).await
    }
}

impl Run<ListComicArchivePayloads<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListComicArchivePayloads<'_>,
    ) -> BaseResult<Vec<(OffsetDateTime, String)>> {
        submit_query!(self.core, list_payloads, oper.team_id, oper.months)
    }
}

impl Step<CommitComicArchive<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CommitComicArchive<'_>,
    ) -> BaseResult<()> {
        commit(context.conn(), oper.entry).await
    }
}
