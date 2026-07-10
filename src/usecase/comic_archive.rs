//! Use case for atomically archiving an active comic.

use time::OffsetDateTime;

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::complex::comic::ComicPermComplex;
use crate::complex::comic_archive::ComicArchiveComplex;
use crate::data::comic_archive::ArchiveComicVal;
use crate::model::comic_archive::ComicArchiveSnapshot;
use crate::model::user::UserToken;
use crate::part::prom::task::{IMAGE_TOPIC, ImageTask};
use crate::part::prom::{Payload, Prom, PromStep};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::comic_archive::ComicArchiveRepoTransactional;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::step::comic_archive::ComicArchiveStep;
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::part::shared::proxy::AsProxyNonTransactional;
use crate::result::{RegularError, RegularResult};
use crate::util::next_snowflake_id;

#[cfg(test)]
mod tests;

/// Archive one active comic, its descendants, and all retained image keys.
pub async fn archive<D, C, R, P>(
    drive: &D,
    repo: &R,
    prom: &P,
    token: UserToken,
    comic_id: String,
) -> RegularResult<ArchiveComicVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Send + Sync,
    R::Transactional: ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + ComicArchiveRepoTransactional<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    ComicPermComplex::can_user_update_info(
        &mut repo.as_proxy(),
        &token.user_id,
        &comic_id,
    )
    .await?;

    let archive_comic_val = drive
        .with_context(async move |context| -> RegularResult<ArchiveComicVal> {
            let repo = repo.derive_transactional().await;
            let comic_archive_snapshot = repo
                .advance(context, &ComicArchiveStep::lock_snapshot(&comic_id))
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

                prom.advance(
                    context,
                    &PromStep::append(
                        &image_delete_id,
                        IMAGE_TOPIC,
                        Payload::Image(ImageTask::Delete {
                            object_key: &image_key,
                        }),
                        &archived_at,
                    ),
                )
                .await?;
            }

            repo.advance(
                context,
                &ComicArchiveStep::commit(&comic_archive_write),
            )
            .await?;

            Ok(ArchiveComicVal { archived_comic_id })
        })
        .await?;

    Ok(archive_comic_val)
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
