//! RDB-backed atomic comic archive repository.

use std::collections::HashMap;

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use poprako_transactional::advance::Advance;

use crate::model::assignment_model;
use crate::model::chapter_model;
use crate::model::comic_archive_model;
use crate::model::comic_model;
use crate::model::page_model;
use crate::model::unit_model;
use crate::model::user_model;
use crate::model::workset_model;
use crate::part::repo::comic_archive::ComicArchiveRepoTransactional;
use crate::part::repo::step::comic_archive::{Commit, LockSnapshot};
use crate::part_impl::repo::rdb_impl::RdbRepoTransactional;
use crate::part_impl::repo::rdb_impl::entity::assignment::AssignmentRow;
use crate::part_impl::repo::rdb_impl::entity::chapter::ChapterRow;
use crate::part_impl::repo::rdb_impl::entity::comic::ComicRow;
use crate::part_impl::repo::rdb_impl::entity::comic_archive::{
    ArchivedChapterEntry, ArchivedComicEntry, ArchivedTranslationEntry,
};
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
use crate::part_impl::repo::rdb_impl::schema::t_page::dsl::{
    f_chapter_id as page_chapter_id, f_id as page_id, f_index as page_index,
    t_page,
};
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{
    f_index as unit_index, f_page_id as unit_page_id, t_unit,
};
use crate::part_impl::repo::rdb_impl::schema::t_user::dsl::{
    f_id as user_id, t_user,
};
use crate::part_impl::repo::rdb_impl::schema::t_workset::dsl::{
    f_id as workset_id, t_workset,
};
use crate::part_impl::repo::rdb_impl::schema::{
    t_archived_chapter, t_archived_comic, t_archived_translation,
};
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{RegularError, RegularResult};

impl ComicArchiveRepoTransactional<RdbContext> for RdbRepoTransactional {}

/// Lock every active descendant needed by an archive transaction.
async fn lock_snapshot(
    conn: &mut RdbConn,
    source_comic_id: &str,
) -> RegularResult<comic_archive_model::Snapshot> {
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

    let comic_info: comic_model::Info = comic_row.into();

    let workset_row: WorksetRow = t_workset
        .filter(workset_id.eq(&comic_info.workset_id))
        .select(WorksetRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-workset-not-found"))?;

    let workset_info: workset_model::Info = workset_row.into();

    let chapter_rows: Vec<ChapterRow> = t_chapter
        .filter(chapter_comic_id.eq(&comic_info.id))
        .select(ChapterRow::as_select())
        .order_by(chapter_id.asc())
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    let chapter_infos: Vec<chapter_model::Info> = chapter_rows
        .into_iter()
        .map(chapter_model::Info::try_from)
        .collect::<RegularResult<Vec<_>>>()?;

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
        .map(assignment_model::Info::try_from)
        .collect::<RegularResult<Vec<_>>>()?;

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
            let user_info: user_model::Info = user_row.into();

            (user_info.id.clone(), user_info)
        })
        .collect::<HashMap<_, _>>();

    let page_rows: Vec<PageRow> = t_page
        .filter(page_chapter_id.eq_any(&source_chapter_ids))
        .select(PageRow::as_select())
        .order_by((page_chapter_id.asc(), page_index.asc(), page_id.asc()))
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    let page_infos: Vec<page_model::Info> =
        page_rows.into_iter().map(Into::into).collect::<Vec<_>>();

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

    let unit_infos: Vec<unit_model::Info> =
        unit_rows.into_iter().map(Into::into).collect::<Vec<_>>();

    let mut assignment_infos_by_chapter: HashMap<
        String,
        Vec<assignment_model::Info>,
    > = HashMap::new();

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
            .push(comic_archive_model::PageSnapshot {
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

            comic_archive_model::ChapterSnapshot {
                chapter_info,
                assignment_infos,
                page_snapshots,
            }
        })
        .collect();

    Ok(comic_archive_model::Snapshot {
        comic_info,
        workset_info,
        chapter_snapshots,
    })
}

/// Insert archive rows and remove the active comic subtree without touching workset counters.
async fn commit(
    conn: &mut RdbConn,
    comic_archive_write: &comic_archive_model::Write,
) -> RegularResult<()> {
    //
    let comic_entry =
        ArchivedComicEntry::from(&comic_archive_write.comic_record);

    diesel::insert_into(t_archived_comic::table)
        .values(&comic_entry)
        .execute(conn)
        .await
        .map_err(diesel)?;

    let chapter_entries = comic_archive_write
        .chapter_records
        .iter()
        .map(ArchivedChapterEntry::from)
        .collect::<Vec<_>>();

    if !chapter_entries.is_empty() {
        diesel::insert_into(t_archived_chapter::table)
            .values(&chapter_entries)
            .execute(conn)
            .await
            .map_err(diesel)?;
    }

    let translation_entries = comic_archive_write
        .translation_records
        .iter()
        .map(ArchivedTranslationEntry::from)
        .collect::<Vec<_>>();

    if !translation_entries.is_empty() {
        diesel::insert_into(t_archived_translation::table)
            .values(&translation_entries)
            .execute(conn)
            .await
            .map_err(diesel)?;
    }

    diesel::delete(t_assignment_invitation.filter(
        invitation_chapter_id.eq_any(&comic_archive_write.source_chapter_ids),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(t_assignment.filter(
        assignment_chapter_id.eq_any(&comic_archive_write.source_chapter_ids),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_unit
            .filter(unit_page_id.eq_any(&comic_archive_write.source_page_ids)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(t_page.filter(
        page_chapter_id.eq_any(&comic_archive_write.source_chapter_ids),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_chapter
            .filter(chapter_id.eq_any(&comic_archive_write.source_chapter_ids)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_comic.filter(comic_id.eq(&comic_archive_write.source_comic_id)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    Ok(())
}

#[async_trait]
impl<'a> Advance<LockSnapshot<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &LockSnapshot<'a>,
    ) -> RegularResult<comic_archive_model::Snapshot> {
        lock_snapshot(context.conn(), step.comic_id).await
    }
}

#[async_trait]
impl<'a> Advance<Commit<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Commit<'a>,
    ) -> RegularResult<()> {
        commit(context.conn(), step.comic_archive_write).await
    }
}

#[cfg(all(test, feature = "repo"))]
mod tests;
