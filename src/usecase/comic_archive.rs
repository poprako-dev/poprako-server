//! Use cases for immutable comic archives.

use std::collections::BTreeMap;

use poprako_orchestra::{Nucl, run_proxy};
use poprako_orchestra_extra::prom::oper::DeferBatch;
use poprako_orchestra_extra::prom::task::Task;
use time::OffsetDateTime;
use tracing::instrument;

use crate::complex::comic_archive::{ComicArchiveComplex, ComicArchivePermComplex};
use crate::data::comic_archive::{ArchiveComicPayload, ExportComicArchivesParams, ExportComicArchivesPayload};
use crate::model::user::UserToken;
use crate::part::prom::Prom;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::comic_archive::ComicArchiveRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::comic_archive::{CommitComicArchive, GetComicArchiveSnapshotExcluded, ListComicArchivePayloads};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseResult, accept};
use crate::util::next_snowflake_id;
use crate::value::comic_archive::ComicArchiveMonth;

#[cfg(test)]
mod tests;

/// Exports selected retained UTC month slots for one team.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn export<C, R>(
    repo: &R,
    token: UserToken,
    team_id: String,
    params: ExportComicArchivesParams,
) -> BaseResult<ExportComicArchivesPayload>
where
    R: ComicArchiveRepo<C> + MemberRepo<C> + Sync,
{
    ComicArchivePermComplex::ensure_user_can_export(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &team_id,
    )
    .await?;

    let months = ComicArchiveMonth::parse_retained(
        params.months,
        OffsetDateTime::now_utc(),
    )?;

    let records = repo
        .run(&ListComicArchivePayloads {
            team_id: &team_id,
            months: &months,
        })
        .await?;

    let mut exports = months
        .iter()
        .map(|month| (month.label.clone(), Vec::new()))
        .collect::<BTreeMap<_, _>>();

    for (created_at, archived_payload) in records {
        //
        let month = months
            .iter()
            .find(|month| created_at >= month.start && created_at < month.end);

        let Some(month) = month else {
            continue;
        };

        exports
            .entry(month.label.clone())
            .or_default()
            .push(archived_payload);
    }

    accept(ExportComicArchivesPayload(exports))
}

/// Archive one active comic, its descendants, and all retained image keys.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn archive<N, C, R, P>(
    nucl: &N,
    repo: &R,
    prom: &P,
    token: UserToken,
    comic_id: String,
) -> BaseResult<ArchiveComicPayload>
where
    N: Nucl<Context = C, Error = BaseError>,
    R: ComicRepo<C>
        + ComicArchiveRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    ComicArchivePermComplex::ensure_user_can_archive(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &comic_id,
    )
    .await?;

    let archive_comic_val = nucl
        .coord(async move |context| {
            //
            let comic_archive_snapshot = repo
                .step(
                    context,
                    &GetComicArchiveSnapshotExcluded {
                        comic_id: &comic_id,
                    },
                )
                .await?;

            let archived_at = OffsetDateTime::now_utc();

            let (comic_archive_entry, image_keys) =
                ComicArchiveComplex::prepare_entry(
                    comic_archive_snapshot,
                    token.user_id,
                    archived_at,
                )
                .await?;

            let archived_comic_id = comic_archive_entry.record.id.clone();

            let mut delete_ids = Vec::new();

            let mut delete_payloads = Vec::new();

            for image_key in image_keys {
                //
                delete_ids.push(next_snowflake_id());

                delete_payloads.push(TaskPayload::Image(image::ImagePayload::Delete {
                    object_key: image_key,
                }));
            }

            let delete_tasks: Vec<Task<'_, String, TaskPayload>> = delete_ids
                .iter()
                .zip(delete_payloads.iter())
                .map(|(id, payload)| Task {
                    id,
                    payload,
                    delay: None,
                })
                .collect();

            prom.step(context, &DeferBatch::new(&delete_tasks)).await?;

            repo.step(
                context,
                &CommitComicArchive {
                    entry: &comic_archive_entry,
                },
            )
            .await?;

            accept(ArchiveComicPayload { archived_comic_id })
        })
        .await?;

    accept(archive_comic_val)
}

// TODO: export
