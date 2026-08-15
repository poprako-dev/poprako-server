//! RDB-backed atomic comic archive repository.

// Persistent archive commit operation.
mod commit;
// Permanent archive payload query.
mod payload;

/// Comic archive RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use std::collections::HashMap;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

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
use crate::part::repo::oper::comic_archive::{
    CommitComicArchive, DeleteComicArchives, GetComicArchiveSnapshotExcluded,
    ListComicArchivePayloads,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::assignment::AssignmentInfoRow;
use crate::part_impl::repo::rdb_impl::entity::chapter::ChapterInfoRow;
use crate::part_impl::repo::rdb_impl::entity::comic::ComicInfoRow;
use crate::part_impl::repo::rdb_impl::entity::page::PageInfoRow;
use crate::part_impl::repo::rdb_impl::entity::unit::UnitInfoRow;
use crate::part_impl::repo::rdb_impl::entity::user::UserInfoRow;
use crate::part_impl::repo::rdb_impl::entity::workset::WorksetInfoRow;
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
    f_page_id as unit_page_id, t_unit,
};
use crate::part_impl::repo::rdb_impl::schema::t_user::dsl::{
    f_id as user_id, t_user,
};
use crate::part_impl::repo::rdb_impl::schema::t_workset::dsl::{
    f_id as workset_id, t_workset,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;
use crate::shared::{RdbConn, RdbContext};

// Standardize chain-corruption failures for unit graph validation.
fn corrupt_unit_chain_err() -> BaseError {
    //
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

    let (mut current_id, mut visible_infos) = (
        Some((*head_id).to_string()),
        Vec::with_capacity(infos_by_id.len()),
    );

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

/// Lock every active descendant needed by an archive transaction.
#[instrument(level = "info", skip_all)]
// Build a full snapshot of all descendants and lock them for commit safety.
async fn get_snapshot_excluded(
    conn: &mut RdbConn,
    source_comic_id: &str,
) -> BaseRest<ComicArchiveSnapshot> {
    //
    let comic_row = t_comic
        .filter(comic_id.eq(source_comic_id))
        .select(ComicInfoRow::as_select())
        .for_update()
        .get_result::<ComicInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(comic_row) = comic_row else {
        //
        let message = trl("error-comic-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            comic_id = %source_comic_id,
            operation = "get comic archive snapshot",
            "expected comic archive error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    };

    let comic_info = TryInto::<ComicInfo>::try_into(comic_row)?;

    let workset_row = t_workset
        .filter(workset_id.eq(&comic_info.workset_id))
        .select(WorksetInfoRow::as_select())
        .for_update()
        .get_result::<WorksetInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(workset_row) = workset_row else {
        //
        let message = trl("error-workset-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            comic_id = %source_comic_id,
            workset_id = %comic_info.workset_id,
            operation = "get comic archive snapshot",
            "expected comic archive error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    };

    let workset_info = Into::<WorksetInfo>::into(workset_row);

    let chapter_rows = t_chapter
        .filter(chapter_comic_id.eq(&comic_info.id))
        .select(ChapterInfoRow::as_select())
        .order_by(chapter_id.asc())
        .for_update()
        .load::<ChapterInfoRow>(conn)
        .await
        .map_err(diesel)?;

    let chapter_infos = chapter_rows
        .into_iter()
        .map(ChapterInfo::try_from)
        .collect::<BaseRest<Vec<ChapterInfo>>>()?;

    let source_chapter_ids = chapter_infos
        .iter()
        .map(|chapter_info| chapter_info.id.clone())
        .collect::<Vec<_>>();

    let _ = t_assignment_invitation
        .filter(invitation_chapter_id.eq_any(&source_chapter_ids))
        .select(invitation_id)
        .for_update()
        .load::<String>(conn)
        .await
        .map_err(diesel)?;

    let assignment_rows = t_assignment
        .filter(assignment_chapter_id.eq_any(&source_chapter_ids))
        .select(AssignmentInfoRow::as_select())
        .for_update()
        .load::<AssignmentInfoRow>(conn)
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

    let user_rows = t_user
        .filter(user_id.eq_any(&assigned_user_ids))
        .select(UserInfoRow::as_select())
        .for_update()
        .load::<UserInfoRow>(conn)
        .await
        .map_err(diesel)?;

    let user_infos = user_rows
        .into_iter()
        .map(|user_row| {
            //
            let user_info = TryInto::<UserInfo>::try_into(user_row)?;

            Ok((user_info.id.clone(), user_info))
        })
        .collect::<BaseRest<HashMap<_, _>>>()?;

    let page_rows = t_page
        .filter(page_chapter_id.eq_any(&source_chapter_ids))
        .select(PageInfoRow::as_select())
        .order_by((page_chapter_id.asc(), page_index.asc(), page_id.asc()))
        .for_update()
        .load::<PageInfoRow>(conn)
        .await
        .map_err(diesel)?;

    let page_infos = page_rows
        .into_iter()
        .map(TryInto::try_into)
        .collect::<BaseRest<Vec<PageInfo>>>()?;

    let source_page_ids = page_infos
        .iter()
        .map(|page_info| page_info.id.clone())
        .collect::<Vec<_>>();

    let unit_rows = t_unit
        .filter(unit_page_id.eq_any(&source_page_ids))
        .select(UnitInfoRow::as_select())
        .order_by(unit_page_id.asc())
        .for_update()
        .load::<UnitInfoRow>(conn)
        .await
        .map_err(diesel)?;

    let (unit_infos, mut assignment_infos_by_chapter) = (
        unit_rows
            .into_iter()
            .map(Into::into)
            .collect::<Vec<UnitInfo>>(),
        HashMap::<String, Vec<AssignmentInfo>>::new(),
    );

    for mut assignment_info in assignment_infos {
        //
        let Some(user_info) = user_infos.get(&assignment_info.user_id) else {
            //
            let message = trl("error-user-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %message,
                comic_id = %source_comic_id,
                chapter_id = %assignment_info.chapter_id,
                assignment_id = %assignment_info.id,
                user_id = %assignment_info.user_id,
                operation = "assemble comic archive snapshot",
                "expected comic archive error",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message,
            });
        };

        assignment_info.user = Some(user_info.clone());

        assignment_infos_by_chapter
            .entry(assignment_info.chapter_id.clone())
            .or_default()
            .push(assignment_info);
    }

    let mut unit_infos_by_page = HashMap::new();

    for unit_info in unit_infos {
        //
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
            let (assignment_infos, page_snapshots) = (
                assignment_infos_by_chapter
                    .remove(&chapter_info.id)
                    .unwrap_or_default(),
                page_snapshots_by_chapter
                    .remove(&chapter_info.id)
                    .unwrap_or_default(),
            );

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

impl<L> Step<GetComicArchiveSnapshotExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: poprako_orchestra::Level + Send,
    L: poprako_orchestra::AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Use base errors for snapshot reads in comic archive transactions.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Resolve the snapshot while holding transaction locks.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetComicArchiveSnapshotExcluded<'_>,
    ) -> BaseRest<ComicArchiveSnapshot> {
        get_snapshot_excluded(context.conn(), oper.comic_id).await
    }
}

impl Run<ListComicArchivePayloads<'_>> for HybRepo {
    // Use base errors for payload-list operations.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Route to shared payload query with team-month filters.
    async fn run(
        &self,
        oper: &ListComicArchivePayloads<'_>,
    ) -> BaseRest<Vec<(OffsetDateTime, String)>> {
        //
        submit_query!(
            self.core,
            payload::list_payloads,
            oper.team_id,
            oper.months
        )
    }
}

impl<L> Step<CommitComicArchive<'_>, RdbContext<L>> for HybRepo
where
    L: poprako_orchestra::Level + Send,
    L: poprako_orchestra::AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Use base errors for commit operations.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Persist archive entry, clear sources, and retain the source comic.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CommitComicArchive<'_>,
    ) -> BaseRest<()> {
        commit::commit(context.conn(), oper.entry).await
    }
}

impl<L> Step<DeleteComicArchives<'_>, RdbContext<L>> for HybRepo
where
    L: poprako_orchestra::Level + Send,
    L: poprako_orchestra::AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Use base errors for comic-archive cleanup during hard deletion.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Delete every archive record associated with a source comic.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeleteComicArchives<'_>,
    ) -> BaseRest<()> {
        //
        use crate::part_impl::repo::rdb_impl::schema::t_comic_archive::dsl::{
            f_source_comic_id, t_comic_archive,
        };

        diesel::delete(
            t_comic_archive.filter(f_source_comic_id.eq(oper.source_comic_id)),
        )
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        accept(())
    }
}
