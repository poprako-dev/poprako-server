//! RDB-backed atomic comic archive repository.

// Persistent archive commit operation.
mod commit;
// Permanent archive payload query.
mod payload;
// Ordered Page and Unit snapshot loading and assembly.
mod pages;
// Independent composite page snapshots.
mod artwork;

/// Comic archive RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use std::collections::HashMap;

use diesel::prelude::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
    SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::part::nucl::ReptRead;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::chapter_workflow_record::ChapterWorkflowRecordInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::comic_archive::{ComicArchiveChapterSnapshot, ComicArchivePageArtworkSnapshot, ComicArchiveSnapshot};
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::UnitInfo;
use crate::model::read::proj::user::UserInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::part::repo::oper::comic_archive::{CommitComicArchive, GetComicArchiveSnapshotExcluded, ListComicArchivePayloads};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::assignment::AssignmentInfoRow;
use crate::part_impl::repo::rdb_impl::entity::chapter::ChapterInfoRow;
use crate::part_impl::repo::rdb_impl::entity::chapter_workflow_record::ChapterWorkflowRecordInfoRow;
use crate::part_impl::repo::rdb_impl::entity::comic::ComicInfoRow;
use crate::part_impl::repo::rdb_impl::entity::user::UserInfoRow;
use crate::part_impl::repo::rdb_impl::entity::workset::WorksetInfoRow;
use crate::part_impl::repo::rdb_impl::schema::t_assignment::dsl::{f_chapter_id as assignment_chapter_id, t_assignment};
use crate::part_impl::repo::rdb_impl::schema::t_assignment_invitation::dsl::{f_chapter_id as invitation_chapter_id, f_id as invitation_id, t_assignment_invitation};
use crate::part_impl::repo::rdb_impl::schema::t_chapter::dsl::{f_comic_id as chapter_comic_id, f_id as chapter_id, t_chapter};
use crate::part_impl::repo::rdb_impl::schema::t_chapter_workflow_record::dsl::{f_chapter_id as workflow_record_chapter_id, f_created_at as workflow_record_created_at, f_id as workflow_record_id, t_chapter_workflow_record};
use crate::part_impl::repo::rdb_impl::schema::t_comic::dsl::{f_deleted_at as comic_deleted_at, f_id as comic_id, t_comic};
use crate::part_impl::repo::rdb_impl::schema::t_user::dsl::{f_id as user_id, t_user};
use crate::part_impl::repo::rdb_impl::schema::t_workset::dsl::{f_id as workset_id, t_workset};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;
use crate::shared::RdbContext;

// Lock and load the root Comic and its Workset for an archive snapshot.
async fn load_archive_root(
    conn: &mut RdbConn,
    source_comic_id: &str,
) -> BaseRest<(ComicInfo, WorksetInfo)> {
    //
    let comic_row = t_comic
        .filter(comic_id.eq(source_comic_id))
        .filter(comic_deleted_at.is_null())
        .select(ComicInfoRow::as_select())
        .for_update()
        .get_result::<ComicInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(comic_row) = comic_row else {
        //
        let msg = trl("error-comic-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_msg = %msg,
            comic_id = %source_comic_id,
            operation = "get comic archive snapshot",
            "expected comic archive error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            msg,
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
        let msg = trl("error-workset-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_msg = %msg,
            comic_id = %source_comic_id,
            workset_id = %comic_info.workset_id,
            operation = "get comic archive snapshot",
            "expected comic archive error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            msg,
        });
    };

    let workset_info = WorksetInfo::try_from(workset_row)?;

    accept((comic_info, workset_info))
}

// Lock and load the source Comic's ordered Chapters.
async fn load_archive_chapters(
    conn: &mut RdbConn,
    source_comic_id: &str,
) -> BaseRest<Vec<ChapterInfo>> {
    //
    let chapter_rows = t_chapter
        .filter(chapter_comic_id.eq(source_comic_id))
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

    accept(chapter_infos)
}

// Lock and load workflow records, invitations, assignments, and assignees.
async fn load_archive_chapter_relations(
    conn: &mut RdbConn,
    source_chapter_ids: &[String],
) -> BaseRest<(
    Vec<ChapterWorkflowRecordInfo>,
    Vec<AssignmentInfo>,
    HashMap<String, UserInfo>,
)> {
    //
    let workflow_record_rows = t_chapter_workflow_record
        .filter(workflow_record_chapter_id.eq_any(source_chapter_ids))
        .select(ChapterWorkflowRecordInfoRow::as_select())
        .order_by((
            workflow_record_chapter_id.asc(),
            workflow_record_created_at.asc(),
            workflow_record_id.asc(),
        ))
        .for_update()
        .load::<ChapterWorkflowRecordInfoRow>(conn)
        .await
        .map_err(diesel)?;

    let workflow_record_infos = workflow_record_rows
        .into_iter()
        .map(ChapterWorkflowRecordInfo::try_from)
        .collect::<BaseRest<Vec<_>>>()?;

    let _ = t_assignment_invitation
        .filter(invitation_chapter_id.eq_any(source_chapter_ids))
        .select(invitation_id)
        .for_update()
        .load::<String>(conn)
        .await
        .map_err(diesel)?;

    let assignment_rows = t_assignment
        .filter(assignment_chapter_id.eq_any(source_chapter_ids))
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

    accept((workflow_record_infos, assignment_infos, user_infos))
}

// Loaded descendants needed to assemble Chapter snapshots.
struct ArchiveChapterParts {
    // Chapters included in the archive.
    chapter_infos: Vec<ChapterInfo>,

    // Chapter stage history.
    workflow_record_infos: Vec<ChapterWorkflowRecordInfo>,
    // Chapter assignments.
    assignment_infos: Vec<AssignmentInfo>,
    // Assignment users indexed by ID.
    users_by_id: HashMap<String, UserInfo>,

    // Ordered Pages included in the archive.
    page_infos: Vec<PageInfo>,
    // Units belonging to the selected Pages.
    unit_infos: Vec<UnitInfo>,
    // Current review details belonging to the selected Chapters.
    page_artwork_snapshots:
        HashMap<String, Vec<ComicArchivePageArtworkSnapshot>>,
}

// Assemble loaded archive descendants into Chapter snapshots.
fn assemble_chapter_snapshots(
    source_comic_id: &str,
    coord_fields: ArchiveChapterParts,
) -> BaseRest<Vec<ComicArchiveChapterSnapshot>> {
    //
    let ArchiveChapterParts {
        chapter_infos,
        workflow_record_infos,
        assignment_infos,
        users_by_id,
        page_infos,
        unit_infos,
        mut page_artwork_snapshots,
    } = coord_fields;

    let mut assignment_infos_by_chapter =
        HashMap::<String, Vec<AssignmentInfo>>::new();

    for mut assignment_info in assignment_infos {
        //
        let Some(user_info) = users_by_id.get(&assignment_info.user_id) else {
            //
            let msg = trl("error-user-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_msg = %msg,
                comic_id = %source_comic_id,
                chapter_id = %assignment_info.chapter_id,
                assignment_id = %assignment_info.id,
                user_id = %assignment_info.user_id,
                operation = "assemble comic archive snapshot",
                "expected comic archive error",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                msg,
            });
        };

        assignment_info.user = Some(user_info.clone());

        assignment_infos_by_chapter
            .entry(assignment_info.chapter_id.clone())
            .or_default()
            .push(assignment_info);
    }

    let mut workflow_record_infos_by_chapter = HashMap::new();

    for workflow_record_info in workflow_record_infos {
        //
        workflow_record_infos_by_chapter
            .entry(workflow_record_info.chapter_id.clone())
            .or_insert_with(Vec::new)
            .push(workflow_record_info);
    }

    let mut page_snapshots_by_chapter =
        pages::assemble_page_snapshots(page_infos, unit_infos)?;

    let chapter_snapshots = chapter_infos
        .into_iter()
        .map(|chapter_info| {
            //
            let (assignment_infos, workflow_record_infos, page_snapshots) = (
                assignment_infos_by_chapter
                    .remove(&chapter_info.id)
                    .unwrap_or_default(),
                workflow_record_infos_by_chapter
                    .remove(&chapter_info.id)
                    .unwrap_or_default(),
                page_snapshots_by_chapter
                    .remove(&chapter_info.id)
                    .unwrap_or_default(),
            );

            ComicArchiveChapterSnapshot {
                page_artwork_snapshots: page_artwork_snapshots
                    .remove(&chapter_info.id)
                    .unwrap_or_default(),
                chapter_info,
                assignment_infos,
                workflow_record_infos,
                page_snapshots,
            }
        })
        .collect();

    accept(chapter_snapshots)
}

/// Lock every active descendant needed by an archive transaction.
// Build a full snapshot of all descendants and lock them for commit safety.
#[instrument(level = "info", skip_all)]
async fn get_snapshot_excluded(
    conn: &mut RdbConn,
    source_comic_id: &str,
) -> BaseRest<ComicArchiveSnapshot> {
    //
    let (comic_info, workset_info) =
        load_archive_root(conn, source_comic_id).await?;

    let chapter_infos = load_archive_chapters(conn, &comic_info.id).await?;

    let source_chapter_ids = chapter_infos
        .iter()
        .map(|chapter_info| chapter_info.id.clone())
        .collect::<Vec<_>>();

    let (workflow_record_infos, assignment_infos, user_infos) =
        load_archive_chapter_relations(conn, &source_chapter_ids).await?;

    let (page_infos, unit_infos) =
        pages::load_archive_pages(conn, &source_chapter_ids).await?;

    let page_artwork_snapshots =
        artwork::load_archive_page_artworks(conn, &source_chapter_ids).await?;

    let chapter_snapshots = assemble_chapter_snapshots(
        source_comic_id,
        ArchiveChapterParts {
            chapter_infos,
            workflow_record_infos,
            assignment_infos,
            users_by_id: user_infos,
            page_infos,
            unit_infos,
            page_artwork_snapshots,
        },
    )?;

    accept(ComicArchiveSnapshot {
        comic_info,
        workset_info,
        chapter_snapshots,
    })
}

impl<L> Step<GetComicArchiveSnapshotExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use base errors for snapshot reads in comic archive transactions.
    type Level = ReptRead;

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
            self.rdb_core,
            payload::list_payloads,
            oper.team_id,
            oper.months
        )
    }
}

impl<L> Step<CommitComicArchive<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use base errors for commit operations.
    type Level = ReptRead;

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
