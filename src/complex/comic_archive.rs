//! Pure conversion of active comic snapshots into immutable archive payloads.

use poprako_orchestra::{OperProxy as _, Proxy};
use time::OffsetDateTime;

use poprako_util::time::ToUnixMilli;

use crate::complex::util::check_user_is_team_admin;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::comic_archive::{
    ComicArchiveChapterSnapshot, ComicArchiveRecord, ComicArchiveSnapshot,
};
use crate::model::write::comic_archive::ComicArchiveEntry;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::result::{BaseError, BaseRest, accept};
use crate::util::next_snowflake_id;
use crate::value::comic_archive::{
    ArchivedAssignmentPayload, ArchivedChapterPayload, ArchivedComicPayload,
    ArchivedPagePayload, ArchivedUnitPayload, ArchivedUserPayload,
    ArchivedWorksetPayload,
};

/// Constructs one immutable comic archive record from a fully locked snapshot.
pub struct ComicArchiveComplex;

impl ComicArchiveComplex {
    /// Builds one compressed archive row and deduplicated image keys on Tokio's blocking pool.
    pub async fn prepare_entry(
        comic_archive_snapshot: ComicArchiveSnapshot,
        archiver_id: String,
        archived_at: OffsetDateTime,
    ) -> BaseRest<(ComicArchiveEntry, Vec<String>)> {
        tokio::task::spawn_blocking(move || {
            //
            let image_keys = collect_image_keys(&comic_archive_snapshot);

            let comic_archive_entry =
                build_entry(comic_archive_snapshot, archiver_id, archived_at)?;

            accept((comic_archive_entry, image_keys))
        })
        .await
        .map_err(|error| BaseError::Unrecoverable {
            message: format!(
                "[ComicArchiveComplex::prepare_entry] blocking task failed: {}",
                error
            ),
        })?
    }
}

/// Permission gates for immutable comic archive operations.
pub struct ComicArchivePermComplex;

impl ComicArchivePermComplex {
    /// Verify that the caller is an administrator of the requested team.
    pub async fn ensure_user_can_export<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify that the caller is an administrator of the comic's owning team.
    pub async fn ensure_user_can_archive<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let comic_info = GetComicInfo {
            id: comic_id,
            incls: &[],
        }
        .proxy_on(proxy)
        .await?;

        let workset_info = GetWorksetInfo {
            id: &comic_info.workset_id,
        }
        .proxy_on(proxy)
        .await?;

        check_user_is_team_admin(proxy, user_id, &workset_info.team_id).await
    }
}

// Convert page and unit data while intentionally excluding image metadata.
fn build_page_payloads(
    chapter_snapshot: &ComicArchiveChapterSnapshot,
) -> Vec<ArchivedPagePayload> {
    chapter_snapshot
        .page_snapshots
        .iter()
        .map(|page_snapshot| {
            //
            let page_info = &page_snapshot.page_info;

            ArchivedPagePayload {
                source_page_id: page_info.id.clone(),
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
                        source_unit_id: unit_info.id.clone(),
                        index: index as i32,
                        is_bubble: unit_info.is_bubble,
                        is_proofread: unit_info.is_proofread,
                        x_coord: unit_info.coord.x_coord,
                        y_coord: unit_info.coord.y_coord,
                        translated_text: unit_info.translated_text.clone(),
                        last_translator_id: unit_info
                            .last_translator_id
                            .clone(),
                        proofread_text: unit_info.proofread_text.clone(),
                        last_proofreader_id: unit_info
                            .last_proofreader_id
                            .clone(),
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
) -> BaseRest<ArchivedComicPayload> {
    //
    let comic_info = &comic_archive_snapshot.comic_info;

    let workset_info = &comic_archive_snapshot.workset_info;

    let chapters = comic_archive_snapshot
        .chapter_snapshots
        .iter()
        .map(build_chapter_payload)
        .collect::<BaseRest<Vec<_>>>()?;

    accept(ArchivedComicPayload {
        source_comic_id: comic_info.id.clone(),
        workset: ArchivedWorksetPayload {
            id: workset_info.id.clone(),
            team_id: workset_info.team_id.clone(),
            index: workset_info.index,
            name: workset_info.name.clone(),
            description: workset_info.description.clone(),
            comic_count: workset_info.comic_count,
            comic_next_index: 0,
            created_at: workset_info.created_at.to_unix_milli(),
            updated_at: workset_info.updated_at.to_unix_milli(),
        },
        index: comic_info.index,
        title: comic_info.title.clone(),
        author: comic_info.author.clone(),
        description: comic_info.description.clone(),
        chapter_count: comic_info.chapter_count,
        chapter_next_index: 0,
        creator_id: comic_info.creator_id.clone(),
        last_active_at: comic_info.last_active_at.to_unix_milli(),
        created_at: comic_info.created_at.to_unix_milli(),
        updated_at: comic_info.updated_at.to_unix_milli(),
        chapters,
    })
}

// Convert an assignment and its directly loaded user into archive data.
fn build_assignment_payload(
    assignment_info: &AssignmentInfo,
) -> BaseRest<ArchivedAssignmentPayload> {
    //
    let user_info = assignment_info.user.as_ref().ok_or_else(|| {
        BaseError::Unrecoverable {
            message: "[ComicArchiveComplex::build_assignment_payload] assignment user was not loaded".into(),
        }
    })?;

    accept(ArchivedAssignmentPayload {
        source_assignment_id: assignment_info.id.clone(),
        user_id: assignment_info.user_id.clone(),
        roles: u32::from(assignment_info.roles),
        created_at: assignment_info.created_at.to_unix_milli(),
        updated_at: assignment_info.updated_at.to_unix_milli(),
        user: ArchivedUserPayload {
            id: user_info.id.clone(),
            qid: user_info.qid.clone(),
            nickname: user_info.nickname.clone(),
            avatar_key: user_info.avatar_key.clone(),
            avatar_uploaded: user_info.is_avatar_uploaded,
            avatar_version: user_info.avatar_version,
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
) -> BaseRest<ArchivedChapterPayload> {
    //
    let chapter_info = &chapter_snapshot.chapter_info;

    let assignments = chapter_snapshot
        .assignment_infos
        .iter()
        .map(build_assignment_payload)
        .collect::<BaseRest<Vec<_>>>()?;

    accept(ArchivedChapterPayload {
        source_chapter_id: chapter_info.id.clone(),
        is_pinned: chapter_info.is_pinned,
        index: chapter_info.index,
        subtitle: chapter_info.subtitle.clone(),
        page_count: chapter_info.page_count,
        total_unit_count: chapter_info.total_unit_count,
        translated_unit_count: chapter_info.translated_unit_count,
        proofread_unit_count: chapter_info.proofread_unit_count,
        stages: u32::from(chapter_info.stages),
        creator_id: chapter_info.creator_id.clone(),
        created_at: chapter_info.created_at.to_unix_milli(),
        updated_at: chapter_info.updated_at.to_unix_milli(),
        assignments,
        pages: build_page_payloads(chapter_snapshot),
    })
}

// Collects every current comic or page object key, including reserved uploads.
fn collect_image_keys(
    comic_archive_snapshot: &ComicArchiveSnapshot,
) -> Vec<String> {
    //
    let mut image_keys = Vec::new();

    if let Some(cover_key) = &comic_archive_snapshot.comic_info.cover_key {
        image_keys.push(cover_key.clone());
    }

    for chapter_snapshot in &comic_archive_snapshot.chapter_snapshots {
        for page_snapshot in &chapter_snapshot.page_snapshots {
            if let Some(image_key) = &page_snapshot.page_info.image_key {
                image_keys.push(image_key.clone());
            }
        }
    }

    image_keys.sort();

    image_keys.dedup();

    image_keys
}

// Builds one compressed archive row and source cleanup identifiers.
fn build_entry(
    comic_archive_snapshot: ComicArchiveSnapshot,
    archiver_id: String,
    archived_at: OffsetDateTime,
) -> BaseRest<ComicArchiveEntry> {
    //
    let archived_comic_id = next_snowflake_id();

    let comic_payload = build_comic_payload(&comic_archive_snapshot)?;

    let record = ComicArchiveRecord {
        id: archived_comic_id.clone(),
        team_id: comic_archive_snapshot.workset_info.team_id.clone(),
        archived_payload: serde_json::to_string(&comic_payload).map_err(|error| {
            BaseError::Unrecoverable {
                message: format!(
                    "[ComicArchiveComplex::build_entry] failed to serialize archive payload: {}",
                    error
                ),
            }
        })?,
        archiver_id,
        created_at: archived_at,
    };

    let mut source_chapter_ids = Vec::new();

    let mut source_page_ids = Vec::new();

    for chapter_snapshot in &comic_archive_snapshot.chapter_snapshots {
        //
        source_chapter_ids.push(chapter_snapshot.chapter_info.id.clone());

        source_page_ids.extend(
            chapter_snapshot
                .page_snapshots
                .iter()
                .map(|page_snapshot| page_snapshot.page_info.id.clone()),
        );
    }

    accept(ComicArchiveEntry {
        record,
        source_comic_id: comic_archive_snapshot.comic_info.id,
        source_chapter_ids,
        source_page_ids,
    })
}
