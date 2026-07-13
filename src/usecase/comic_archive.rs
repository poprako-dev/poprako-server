//! Use case for atomically archiving an active comic.

use time::OffsetDateTime;

use poprako_orchestra::{Nucl, run_proxy};
use poprako_orchestra_extra::prom::oper::Defer;
use poprako_orchestra_extra::prom::task::Task;

use crate::complex::comic_archive::{
    ComicArchiveComplex, ComicArchivePermComplex,
};
use crate::data::comic_archive::ArchiveComicPayload;
use crate::model::comic_archive::ComicArchiveSnapshot;
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
    ComicArchivePermComplex::can_user_archive(
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
            let comic_archive_snapshot = repo
                .step(
                    context,
                    &GetComicArchiveSnapshotExcluded {
                        comic_id: &comic_id,
                    },
                )
                .await?;

            let image_keys = collect_image_keys(&comic_archive_snapshot);

            let archived_at = OffsetDateTime::now_utc();

            let comic_archive_write = ComicArchiveComplex::build_write(
                comic_archive_snapshot,
                token.user_id,
                archived_at,
            )?;

            let archived_comic_id = comic_archive_write.comic_record.id.clone();

            for image_key in image_keys {
                let image_delete_id = next_snowflake_id();

                let payload = Payload::Image(image::Payload::Delete {
                    object_key: image_key,
                });

                let task = Task {
                    id: &image_delete_id,
                    payload: &payload,
                    delay: None,
                };

                prom.step(context, &Defer::new(task)).await?;
            }

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

/// Collect every current comic or page object key, including reserved uploads.
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
