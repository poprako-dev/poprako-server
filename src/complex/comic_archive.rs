//! Pure conversion of active comic snapshots into immutable archive payloads.

use time::OffsetDateTime;

use poprako_util::i18n::trl;
use poprako_util::time::ToUnixMilli as _;

use crate::complex::util::check_user_is_team_admin;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::comic_archive::{
    ComicArchiveChapterSnapshot, ComicArchiveRecord, ComicArchiveSnapshot,
};
use crate::model::read::proj::member::MemberInfo;
use crate::model::write::comic_archive::ComicArchiveEntry;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::comic_archive::workflow_record::ArchivedChapterWorkflowRecordDetail;
use crate::value::comic_archive::{
    ArchivedAssignmentPayload, ArchivedChapterPayload,
    ArchivedChapterWorkflowRecordPayload, ArchivedComicPayload,
    ArchivedPagePayload, ArchivedUnitPayload, ArchivedUserPayload,
    ArchivedWorksetPayload,
};

/// Constructs one immutable comic archive record from a fully locked snapshot.
pub struct ComicArchiveComplex;

impl ComicArchiveComplex {
    /// Rejects archive attempts until every retained chapter has published.
    pub fn ensure_snapshot_archivable(
        comic_archive_snapshot: &ComicArchiveSnapshot,
    ) -> BaseRest<()> {
        //
        if comic_archive_snapshot.comic_info.archived_at.is_some() {
            //
            let message = trl("error-comic-archived");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %message,
                "expected comic archive error",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message,
            });
        }

        let is_archivable =
            !comic_archive_snapshot.chapter_snapshots.is_empty()
                && comic_archive_snapshot.chapter_snapshots.iter().all(
                    |chapter_snapshot| {
                        //
                        chapter_snapshot
                            .chapter_info
                            .stages
                            .has_phase(Stage::Publish, StagePhase::Completed)
                    },
                );

        if is_archivable {
            return accept(());
        }

        let message = trl("error-comic-archive-incomplete");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %message,
            "expected comic archive error",
        );

        Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        })
    }

    /// Builds one compressed archive row on Tokio's blocking pool.
    pub async fn prepare_entry(
        comic_archive_snapshot: ComicArchiveSnapshot,
        archiver_id: String,
        archived_at: OffsetDateTime,
    ) -> BaseRest<ComicArchiveEntry> {
        //
        tokio::task::spawn_blocking(move || {
            //
            let comic_archive_entry =
                build_entry(comic_archive_snapshot, archiver_id, archived_at)?;

            accept(comic_archive_entry)
        })
        .await
        .map_err(|error| {
            //
            tracing::error!(
                operation = "prepare_comic_archive",
                sdk_err = ?error,
                "Tokio SDK blocking task error",
            );

            BaseError::Unrecoverable {
                    message: format!(
                        "[ComicArchiveComplex::prepare_entry] blocking task failed: {}",
                        error,
                ),
            }
        })?
    }
}

/// Permission gates for immutable comic archive operations.
pub struct ComicArchivePermComplex;

impl ComicArchivePermComplex {
    /// Verify that the caller is an administrator of the requested team.
    pub fn ensure_user_can_export(member_info: &MemberInfo) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }

    /// Verify that the caller is an administrator of the comic's owning team.
    pub fn ensure_user_can_archive(member_info: &MemberInfo) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }
}

// Convert page and unit data while intentionally excluding image metadata.
fn build_page_payloads(
    chapter_snapshot: &ComicArchiveChapterSnapshot,
) -> Vec<ArchivedPagePayload<'_>> {
    //
    chapter_snapshot
        .page_snapshots
        .iter()
        .map(|page_snapshot| {
            //
            let page_info = &page_snapshot.page_info;

            ArchivedPagePayload {
                source_page_id: &page_info.id,
                index: page_info.index,
                total_unit_count: page_info.total_unit_count,
                translated_unit_count: page_info.translated_unit_count,
                proofread_unit_count: page_info.proofread_unit_count,
                created_at: page_info.created_at.to_unix_milli(),
                updated_at: page_info.updated_at.to_unix_milli(),
                units: page_snapshot
                    .unit_infos
                    .iter()
                    .enumerate()
                    .map(|(index, unit_info)| ArchivedUnitPayload {
                        source_unit_id: &unit_info.id,
                        index,
                        is_bubble: unit_info.is_bubble,
                        is_proofread: unit_info.is_proofread,
                        x_coord: unit_info.coord.x_coord,
                        y_coord: unit_info.coord.y_coord,
                        translated_text: unit_info.translated_text.as_deref(),
                        last_translator_id: unit_info
                            .last_translator_id
                            .as_deref(),
                        proofread_text: unit_info.proofread_text.as_deref(),
                        last_proofreader_id: unit_info
                            .last_proofreader_id
                            .as_deref(),
                        created_at: unit_info.created_at.to_unix_milli(),
                        updated_at: unit_info.updated_at.to_unix_milli(),
                    })
                    .collect(),
            }
        })
        .collect()
}

// Convert the comic and directly loaded workset into the archive payload.
fn build_comic_payload(
    comic_archive_snapshot: &ComicArchiveSnapshot,
) -> BaseRest<ArchivedComicPayload<'_>> {
    //
    let (comic_info, workset_info) = (
        &comic_archive_snapshot.comic_info,
        &comic_archive_snapshot.workset_info,
    );

    let chapters = comic_archive_snapshot
        .chapter_snapshots
        .iter()
        .map(build_chapter_payload)
        .collect::<BaseRest<Vec<_>>>()?;

    accept(ArchivedComicPayload {
        source_comic_id: &comic_info.id,
        workset: ArchivedWorksetPayload {
            id: &workset_info.id,
            team_id: &workset_info.team_id,
            index: workset_info.index,
            name: &workset_info.name,
            description: workset_info.description.as_deref(),
            comic_count: workset_info.comic_count,
            comic_next_index: 0,
            created_at: workset_info.created_at.to_unix_milli(),
            updated_at: workset_info.updated_at.to_unix_milli(),
        },
        index: comic_info.index,
        title: &comic_info.title,
        author: &comic_info.author,
        description: comic_info.description.as_deref(),
        chapter_count: comic_info.chapter_count,
        chapter_next_index: 0,
        creator_id: &comic_info.creator_id,
        last_active_at: comic_info.last_active_at.to_unix_milli(),
        created_at: comic_info.created_at.to_unix_milli(),
        updated_at: comic_info.updated_at.to_unix_milli(),
        chapters,
    })
}

// Convert an assignment and its directly loaded user into archive data.
fn build_assignment_payload(
    assignment_info: &AssignmentInfo,
) -> BaseRest<ArchivedAssignmentPayload<'_>> {
    //
    let user_info = assignment_info.user.as_ref().ok_or_else(|| {
        //
        BaseError::Unrecoverable {
            message: "[ComicArchiveComplex::build_assignment_payload] assignment user was not loaded".into(),
        }
    })?;

    accept(ArchivedAssignmentPayload {
        source_assignment_id: &assignment_info.id,
        user_id: &assignment_info.user_id,
        roles: u32::from(assignment_info.roles),
        created_at: assignment_info.created_at.to_unix_milli(),
        updated_at: assignment_info.updated_at.to_unix_milli(),
        user: ArchivedUserPayload {
            id: &user_info.id,
            qid: &user_info.qid,
            nickname: &user_info.nickname,
            is_sadmin: user_info.is_sadmin,
            last_active_at: user_info.last_active_at.to_unix_milli(),
            created_at: user_info.created_at.to_unix_milli(),
            updated_at: user_info.updated_at.to_unix_milli(),
        },
    })
}

// Convert a chapter and its assignments into the archive payload.
fn build_chapter_payload(
    chapter_snapshot: &ComicArchiveChapterSnapshot,
) -> BaseRest<ArchivedChapterPayload<'_>> {
    //
    let chapter_info = &chapter_snapshot.chapter_info;

    let assignments = chapter_snapshot
        .assignment_infos
        .iter()
        .map(build_assignment_payload)
        .collect::<BaseRest<Vec<_>>>()?;

    accept(ArchivedChapterPayload {
        source_chapter_id: &chapter_info.id,
        is_pinned: chapter_info.is_pinned,
        index: chapter_info.index,
        subtitle: &chapter_info.subtitle,
        page_count: chapter_info.page_count,
        total_unit_count: chapter_info.total_unit_count,
        translated_unit_count: chapter_info.translated_unit_count,
        proofread_unit_count: chapter_info.proofread_unit_count,
        stages: u32::from(chapter_info.stages),
        creator_id: &chapter_info.creator_id,
        created_at: chapter_info.created_at.to_unix_milli(),
        updated_at: chapter_info.updated_at.to_unix_milli(),
        assignments,
        workflow_records: chapter_snapshot
            .workflow_record_infos
            .iter()
            .map(|record_info| ArchivedChapterWorkflowRecordPayload {
                id: &record_info.id,
                actor_user_id: record_info.actor_user_id.as_deref(),
                kind: record_info.kind,
                payload: ArchivedChapterWorkflowRecordDetail::from(
                    &record_info.payload,
                ),
                created_at: record_info.created_at.to_unix_milli(),
            })
            .collect(),
        pages: build_page_payloads(chapter_snapshot),
    })
}

// Builds one compressed archive row and source cleanup identifiers.
fn build_entry(
    comic_archive_snapshot: ComicArchiveSnapshot,
    archiver_id: String,
    archived_at: OffsetDateTime,
) -> BaseRest<ComicArchiveEntry> {
    //
    let archived_comic_id = next_snowflake_id();

    let archived_payload = {
        //
        let comic_payload = build_comic_payload(&comic_archive_snapshot)?;

        serde_json::to_string(&comic_payload).map_err(|error| {
            //
            tracing::error!(
                operation = "serialize_comic_archive",
                sdk_err = ?error,
                "JSON SDK serialization error",
            );

            BaseError::Unrecoverable {
                    message: format!(
                        "[ComicArchiveComplex::build_entry] failed to serialize archive payload: {}",
                        error,
                ),
            }
        })?
    };

    let ComicArchiveSnapshot {
        comic_info,
        workset_info,
        chapter_snapshots,
    } = comic_archive_snapshot;

    let record = ComicArchiveRecord {
        id: archived_comic_id,
        team_id: workset_info.team_id,
        source_comic_id: comic_info.id,
        archived_payload,
        archiver_id,
        created_at: archived_at,
    };

    let (mut source_chapter_ids, mut source_page_ids) =
        (Vec::new(), Vec::new());

    for chapter_snapshot in chapter_snapshots {
        //
        source_chapter_ids.push(chapter_snapshot.chapter_info.id);

        source_page_ids.extend(
            chapter_snapshot
                .page_snapshots
                .into_iter()
                .map(|page_snapshot| page_snapshot.page_info.id),
        );
    }

    accept(ComicArchiveEntry {
        record,
        source_chapter_ids,
        source_page_ids,
    })
}
