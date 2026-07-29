//! Use case for atomically archiving an active comic.

use poprako_orchestra::{Nucl, run_proxy};
use poprako_orchestra_extra::prom::oper::DeferBatch;
use poprako_orchestra_extra::prom::task::Task;
use time::OffsetDateTime;
use tracing::instrument;

use crate::complex::comic_archive::{
    ComicArchiveComplex, ComicArchivePermComplex,
};
use crate::data::comic_archive::ArchiveComicPayload;
use crate::model::user::UserToken;
use crate::part::prom::Prom;
use crate::part::prom::payload::{Payload, image};
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::comic_archive::ComicArchiveRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::comic_archive::{
    CommitComicArchive, GetComicArchiveSnapshotExcluded,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{RegularError, RegularResult, accept};
use crate::util::next_snowflake_id;

#[cfg(test)]
mod tests;

/// Archive one active comic, its descendants, and all retained image keys.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn archive<N, C, R, P>(
    nucl: &N,
    repo: &R,
    prom: &P,
    token: UserToken,
    comic_id: String,
) -> RegularResult<ArchiveComicPayload>
where
    N: Nucl<Context = C, Error = RegularError>,
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
        .coord(async move |context| -> RegularResult<ArchiveComicPayload> {
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

            let (comic_archive_write, image_keys) =
                ComicArchiveComplex::prepare_write(
                    comic_archive_snapshot,
                    token.user_id,
                    archived_at,
                )
                .await?;

            let archived_comic_id = comic_archive_write.comic_record.id.clone();

            let mut delete_ids = Vec::new();

            let mut delete_payloads = Vec::new();

            for image_key in image_keys {
                delete_ids.push(next_snowflake_id());

                delete_payloads.push(Payload::Image(image::Payload::Delete {
                    object_key: image_key,
                }));
            }

            let delete_tasks: Vec<Task<'_, String, Payload>> = delete_ids
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
                    write: &comic_archive_write,
                },
            )
            .await?;

            Ok(ArchiveComicPayload { archived_comic_id })
        })
        .await?;

    accept(archive_comic_val)
}
