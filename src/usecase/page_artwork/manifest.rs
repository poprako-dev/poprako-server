//! Authoritative composite manifests and their object obligations.

use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::oper::DeleteObjs;

use crate::complex::{
    chapter as chapter_complex, page_artwork as page_artwork_complex,
};
use crate::config::image::ImageConfig;
use crate::data::instr::page_artwork::AllocChapterPageArtworksInstr;
use crate::data::val::page_artwork::AllocChapterPageArtworksVal;
use crate::model::shared::user::UserToken;
use crate::model::write::page_artwork::PageArtworkEntry;
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::PageArtworkImage;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page_artwork::{
    ListPageArtworkInfos, ReplacePageArtworkManifest,
};
use crate::part::repo::page_artwork::PageArtworkRepo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest};
use crate::usecase::page_artwork::access::ensure_user_can_write;
use crate::usecase::page_artwork::upload::alloc_image_slot;
use crate::util::next_business_id;
use crate::value::business_id::BusinessIdKind;

/// Applies only explicit identities; names and hashes never infer page ownership.
#[instrument(
    level = "info",
    skip(nucl, repo, obj_dept, image_config, token, instr),
    fields(actor_user_id = %token.user_id, page_count = instr.pages.len()),
)]
pub async fn alloc_chapter_page_artworks<N, C, R, O>(
    (nucl, repo, obj_dept, image_config): (&N, &R, &O, &ImageConfig),
    token: UserToken,
    chapter_id: String,
    instr: AllocChapterPageArtworksInstr,
) -> BaseRest<AllocChapterPageArtworksVal>
where
    C: Context + Send,
    C::Level: AtLeast<ReptRead>,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    R: ChapterRepo<C>
        + ComicRepo<C>
        + PageArtworkRepo<C>
        + AssignmentRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + Send
        + Sync,
    O: ObjDept<PageArtworkImage, C> + Send + Sync,
{
    let allocated_page_artworks_val = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            chapter_complex::ensure_chapter_writable(&chapter_info)?;

            ensure_user_can_write(
                repo,
                context,
                &chapter_info.id,
                &token.user_id,
            )
            .await?;

            let page_artwork_infos = ListPageArtworkInfos {
                chapter_id: &chapter_info.id,
            }
            .step_on(repo, context)
            .await?;

            page_artwork_complex::ensure_manifest(
                instr
                    .pages
                    .iter()
                    .filter_map(|page| page.page_artwork_id.as_deref()),
                &page_artwork_infos,
            )?;

            let mut entries = Vec::with_capacity(instr.pages.len());

            let mut image_inputs = Vec::with_capacity(instr.pages.len());

            for (index, page) in instr.pages.into_iter().enumerate() {
                //
                let entry = PageArtworkEntry {
                    id: page.page_artwork_id.unwrap_or_else(|| {
                        next_business_id(BusinessIdKind::PageArtwork)
                    }),
                    chapter_id: chapter_info.id.clone(),
                    index,
                    raw_ident: page.raw_ident,
                };

                entries.push(entry);

                image_inputs.push((
                    page.image_hash,
                    page.ext,
                    page.new_byte_len,
                ));
            }

            let removed_ids = page_artwork_infos
                .into_iter()
                .filter(|info| !entries.iter().any(|entry| entry.id == info.id))
                .map(|info| info.id)
                .collect::<Vec<_>>();

            DeleteObjs::<PageArtworkImage>::new(&removed_ids)
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)?;

            ReplacePageArtworkManifest {
                chapter_id: &chapter_info.id,
                entries: &entries,
            }
            .step_on(repo, context)
            .await?;

            let mut allocated_page_artwork_vals =
                Vec::with_capacity(entries.len());

            for (entry, image_input) in entries.iter().zip(image_inputs) {
                //
                let allocated_page_artwork_val = alloc_image_slot(
                    (obj_dept, image_config),
                    context,
                    entry,
                    image_input,
                )
                .await?;

                allocated_page_artwork_vals.push(allocated_page_artwork_val);
            }

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            accept!(AllocChapterPageArtworksVal {
                pages: allocated_page_artwork_vals,
            })
        })
        .await?;

    accept!(allocated_page_artworks_val)
}
