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
use crate::result::{BaseError, BaseRest, accept};
use crate::usecase::page_artwork::access::ensure_user_can_write;
use crate::usecase::page_artwork::upload::alloc_image_slot;
use crate::util::next_snowflake_id;

/// Applies only explicit identities; names and hashes never infer page ownership.
#[instrument(level = "info", skip(nucl, repo, obj_dept, image_config, token, instr), fields(actor_user_id = %token.user_id, page_count = instr.pages.len()))]
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
                &instr.pages,
                &page_artwork_infos,
            )?;

            let page_artwork_entries = instr
                .pages
                .iter()
                .enumerate()
                .map(|(index, page)| PageArtworkEntry {
                    id: page
                        .page_artwork_id
                        .clone()
                        .unwrap_or_else(next_snowflake_id),
                    chapter_id: chapter_info.id.clone(),
                    index,
                    raw_ident: page.raw_ident.clone(),
                })
                .collect::<Vec<_>>();

            let removed_ids = page_artwork_infos
                .iter()
                .filter(|info| {
                    //
                    !page_artwork_entries.iter().any(|page_artwork_entry| {
                        page_artwork_entry.id == info.id
                    })
                })
                .map(|info| info.id.clone())
                .collect::<Vec<_>>();

            DeleteObjs::<PageArtworkImage>::new(&removed_ids)
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)?;

            ReplacePageArtworkManifest {
                chapter_id: &chapter_info.id,
                entries: &page_artwork_entries,
            }
            .step_on(repo, context)
            .await?;

            let mut allocated_page_artwork_vals =
                Vec::with_capacity(page_artwork_entries.len());

            for (page_artwork_entry, page) in
                page_artwork_entries.iter().zip(instr.pages)
            {
                //
                let allocated_page_artwork_val = alloc_image_slot(
                    (obj_dept, image_config),
                    context,
                    page_artwork_entry,
                    (page.image_hash, page.ext, page.new_byte_len),
                )
                .await?;

                allocated_page_artwork_vals.push(allocated_page_artwork_val);
            }

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            accept(AllocChapterPageArtworksVal {
                pages: allocated_page_artwork_vals,
            })
        })
        .await?;

    accept(allocated_page_artworks_val)
}
