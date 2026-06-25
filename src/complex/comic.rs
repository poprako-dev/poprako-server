//! Complex-domain operations for comic entities: identity generation, cover-storage
//! key management, and recursive deletion with related resource cleanup.

use time::OffsetDateTime;
use uuid::Uuid;

use crate::complex::image::ImageComplex;
use crate::part::prom::intention::{IMAGE_TOPIC, ImageIntention};
use crate::part::prom::{Payload, PromStep, PromTransactional};
use crate::part::repo::comic::ComicRepoTransactional;
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::step::workset::WorksetStep;
use crate::part::repo::workset::WorksetRepoTransactional;
use crate::result::RootResult;

/// Domain operations for comic entities.
pub struct ComicComplex;

impl ComicComplex {
    /// Generate a unique, time-ordered comic identifier (e.g. `comic-<uuid-v7>`).
    pub fn gen_id() -> String {
        format!("comic-{}", Uuid::now_v7())
    }

    /// Generate the object-storage key for a comic cover image.
    pub fn gen_cover_key(id: &str, cover_version: i64, file_ext: &str) -> String {
        format!("comic_cover/{}-{}.{}", id, cover_version, file_ext)
    }

    /// Recursively delete a comic and its attached resources: enqueues cover-image
    /// deletion and decrements the parent workset's comic count.
    pub async fn delete_cascade<C, R, P>(
        repo: &R,
        prom: &P,
        context: &mut C,
        id: &str,
    ) -> RootResult<()>
    where
        C: Send,
        R: ComicRepoTransactional<C> + WorksetRepoTransactional<C> + Send + Sync,
        P: PromTransactional<C> + Send + Sync,
    {
        let comic_info = repo
            .advance(context, &ComicStep::get_info_excluded(id))
            .await?;

        if let Some(cover_key) = &comic_info.cover_key
            && comic_info.cover_uploaded
        {
            let delete_id = ImageComplex::gen_delete_id();
            let now = OffsetDateTime::now_utc();

            prom.advance(
                context,
                &PromStep::append(
                    &delete_id,
                    IMAGE_TOPIC,
                    Payload::Image(ImageIntention::Delete {
                        object_key: cover_key.clone(),
                    }),
                    &now,
                ),
            )
            .await?;
        }

        repo.advance(context, &ComicStep::delete(id)).await?;

        repo.advance(
            context,
            &WorksetStep::update_comic_count(&comic_info.workset_id, -1),
        )
        .await?;

        Ok(())
    }
}
