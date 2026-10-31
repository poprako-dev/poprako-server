//! Use cases for immutable comic archives.

// Bounded temporary export construction.
mod export;

#[cfg(test)]
mod tests;

use std::slice;
use std::sync::Arc;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use time::OffsetDateTime;
use tokio::fs::File;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::key::ObjKind as _;
use poprako_obj_dept::oper::{ClearObjs, DeleteObjs};
use poprako_util::i18n::trl;

use crate::complex::comic_archive as comic_archive_complex;
use crate::complex::comic_archive::perm as comic_archive_perm_complex;
use crate::config::comic_archive_export::ComicArchiveExportConfig;
use crate::data::instr::comic_archive::ExportComicArchivesInstr;
use crate::data::val::comic_archive::ArchiveComicVal;
use crate::model::shared::user::UserToken;
use crate::part::nucl::{ReptRead, Serial};
use crate::part::obj_dept::{
    ChapterArtwork, ComicCover, PageArtworkImage, PageSourceImage,
};
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::comic_archive::ComicArchiveRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::comic_archive::{
    CommitComicArchive, GetComicArchiveSnapshotExcluded,
    ListComicArchivePayloads,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::usecase::comic_archive::export::ExportWriter;
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;
use crate::value::comic_archive::ComicArchiveMonth;

/// Complete archive JSON data, generated before response transmission starts.
///
/// The anonymous temporary file is removed when its final handle is dropped.
pub struct ComicArchiveExport {
    /// Rewound file containing only the complete JSON month map.
    file: File,
    /// Exact byte length of the complete JSON month map.
    bytes: u64,
    /// Admission remains occupied until the completed response body is dropped.
    permit: OwnedSemaphorePermit,
}

impl ComicArchiveExport {
    /// Owns a completed artifact and its transfer admission permit.
    #[must_use]
    pub const fn new(
        file: File,
        bytes: u64,
        permit: OwnedSemaphorePermit,
    ) -> Self {
        Self {
            file,
            bytes,
            permit,
        }
    }

    /// Transfers the artifact and admission ownership to its response stream.
    pub fn into_transfer(self) -> (File, u64, OwnedSemaphorePermit) {
        (self.file, self.bytes, self.permit)
    }
}

/// Exports selected retained UTC month slots for one team.
#[instrument(
    level = "info",
    skip(nucl, repo, config, admission, token),
    fields(
        actor_user_id = %token.user_id,
    ),
)]
pub async fn export<N, C, R>(
    (nucl, repo, config, admission): (
        &N,
        &R,
        &ComicArchiveExportConfig,
        &Arc<Semaphore>,
    ),
    token: UserToken,
    team_id: String,
    instr: ExportComicArchivesInstr,
) -> BaseRest<ComicArchiveExport>
where
    C: Context + Send,
    C::Level: AtLeast<ReptRead>,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    R: ComicArchiveRepo<C> + MemberRepo<C> + Sync,
{
    //
    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        return Err(expected_error! {
            ExpectedVariant::Perm,
            message: trl("error-team-admin-required");
            team_id = %team_id,
            user_id = %token.user_id,
            "comic archive export membership missing",
        });
    };

    comic_archive_perm_complex::ensure_user_can_export(&member_info)?;

    let months = ComicArchiveMonth::parse_retained(
        instr.months,
        OffsetDateTime::now_utc(),
    )?;

    let permit =
        Arc::clone(admission).try_acquire_owned().map_err(|error| {
            //
            tracing::warn!(
                operation = "export_comic_archives",
                sdk_error = ?error,
                "archive export capacity unavailable",
            );

            BaseError::Unavailable {
                message: "archive export capacity unavailable".into(),
            }
        })?;

    let (file, bytes) = nucl
        .coord(async move |context| {
            //
            let mut writer = ExportWriter::new(config.artifact_limit_bytes())?;

            writer.write(b"{").await?;

            for (month_index, month) in months.iter().enumerate() {
                //
                if month_index > 0 {
                    writer.write(b",").await?;
                }

                writer.string(&month.label).await?;

                writer.write(b":[").await?;

                let mut after = None::<(OffsetDateTime, String)>;

                let mut first_payload = true;

                loop {
                    //
                    let records = ListComicArchivePayloads {
                        team_id: &team_id,
                        months: slice::from_ref(month),
                        after: after
                            .as_ref()
                            .map(|(created_at, id)| (*created_at, id.as_str())),
                        payload_budget_bytes: config.payload_batch_bytes(),
                    }
                    .step_on(repo, context)
                    .await?;

                    if records.is_empty() {
                        break;
                    }

                    for record in records {
                        //
                        if !first_payload {
                            writer.write(b",").await?;
                        }

                        writer.string(&record.payload).await?;

                        first_payload = false;

                        after = Some((record.created_at, record.id));
                    }
                }

                writer.write(b"]").await?;
            }

            writer.write(b"}").await?;

            writer.finish().await
        })
        .await?;

    accept!(ComicArchiveExport::new(file, bytes, permit))
}

/// Archive one active comic, its descendants, and all retained image keys.
#[instrument(
    level = "info",
    skip(nucl, repo, obj_dept, token),
    fields(
        actor_user_id = %token.user_id,
    ),
)]
pub async fn archive<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    comic_id: String,
) -> BaseRest<ArchiveComicVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<Serial>,
    R: ComicRepo<C>
        + ComicArchiveRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + Send
        + Sync,
    O: ObjDept<PageArtworkImage, C>
        + ObjDept<ChapterArtwork, C>
        + ObjDept<ComicCover, C>
        + ObjDept<PageSourceImage, C>
        + Send
        + Sync,
{
    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::<C>::Run,
        &token.user_id,
        &comic_id,
    )
    .await?;

    comic_archive_perm_complex::ensure_user_can_archive(&member_info)?;

    let archive_comic_val = nucl
        .coord(async move |context| {
            //
            let comic_archive_snapshot = GetComicArchiveSnapshotExcluded {
                comic_id: &comic_id,
            }
            .step_on(repo, context)
            .await?;

            comic_archive_complex::ensure_snapshot_archivable(
                &comic_archive_snapshot,
            )?;

            let archived_at = OffsetDateTime::now_utc();

            let entry = comic_archive_complex::prepare_entry(
                comic_archive_snapshot,
                token.user_id,
                archived_at,
            )
            .await?;

            let comic_cover_ids = ComicCover::ids(std::slice::from_ref(
                &entry.record.source_comic_id,
            ))?;

            ClearObjs::<ComicCover>::new(&comic_cover_ids)
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)?;

            let page_artwork_image_ids =
                PageArtworkImage::ids(&entry.source_page_artwork_ids)?;

            DeleteObjs::<PageArtworkImage>::new(&page_artwork_image_ids)
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)?;

            let page_source_image_ids =
                PageSourceImage::ids(&entry.source_page_source_ids)?;

            DeleteObjs::<PageSourceImage>::new(&page_source_image_ids)
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)?;

            let chapter_artwork_ids =
                ChapterArtwork::ids(&entry.source_chapter_ids)?;

            DeleteObjs::<ChapterArtwork>::new(&chapter_artwork_ids)
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)?;

            CommitComicArchive { entry: &entry }
                .step_on(repo, context)
                .await?;

            let record = entry.record;

            accept!(ArchiveComicVal {
                archived_id: record.id,
            })
        })
        .await?;

    accept!(archive_comic_val)
}
