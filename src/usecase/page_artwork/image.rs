//! Composite image allocation and exact-generation confirmation.

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::key::ObjGen;
use poprako_obj_dept::oper::MarkObjUploaded;

use crate::complex::{
    chapter as chapter_complex, page_artwork as page_artwork_complex,
};
use crate::config::image::ImageConfig;
use crate::data::instr::page_artwork::{
    AllocPageArtworkImageInstr, MarkPageArtworkImageUploadedInstr,
};
use crate::data::val::page_artwork::AllocatedPageArtworkVal;
use crate::model::shared::user::UserToken;
use crate::model::write::page_artwork::{PageArtworkEntry, PageArtworkPatch};
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::PageArtworkImage;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page_artwork::{
    GetPageArtworkInfo, ListPageArtworkInfos, UpdatePageArtworkInfo,
};
use crate::part::repo::page_artwork::PageArtworkRepo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::page_artwork::access::ensure_user_can_write;
use crate::usecase::page_artwork::upload::alloc_image_slot;

/// Replaces one composite image without changing its identity or review issues.
#[instrument(level = "info", skip(nucl, repo, obj_dept, image_config, token, instr), fields(actor_user_id = %token.user_id, image_hash = ?instr.image_hash, ext = ?instr.ext, new_byte_len = instr.new_byte_len))]
pub async fn alloc_image<N, C, R, O>(
    (nucl, repo, obj_dept, image_config): (&N, &R, &O, &ImageConfig),
    token: UserToken,
    id: String,
    instr: AllocPageArtworkImageInstr,
) -> BaseRest<AllocatedPageArtworkVal>
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
    let page_artwork_info = GetPageArtworkInfo { id: &id }.run_on(repo).await?;

    let allocated_page_artwork_val = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &page_artwork_info.chapter_id,
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

            let page_artwork_info = page_artwork_infos
                .into_iter()
                .find(|info| info.id == id)
                .ok_or_else(|| {
                    //
                    page_artwork_complex::error(
                        ExpectedVariant::Args,
                        "error-page-artwork-not-found",
                    )
                })?;

            let page_artwork_patch = PageArtworkPatch {
                id: page_artwork_info.id.clone(),
                raw_ident: instr.raw_ident,
            };

            UpdatePageArtworkInfo {
                update: &page_artwork_patch,
            }
            .step_on(repo, context)
            .await?;

            let page_artwork_entry = PageArtworkEntry {
                id: page_artwork_info.id,
                chapter_id: page_artwork_info.chapter_id,
                index: page_artwork_info.index,
                raw_ident: page_artwork_patch.raw_ident,
            };

            let allocated_page_artwork_val = alloc_image_slot(
                (obj_dept, image_config),
                context,
                &page_artwork_entry,
                (instr.image_hash, instr.ext, Some(instr.new_byte_len)),
            )
            .await?;

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            accept(allocated_page_artwork_val)
        })
        .await?;

    accept(allocated_page_artwork_val)
}

/// Confirms the current image generation while preserving all review issues.
#[instrument(level = "info", skip(nucl, repo, obj_dept, token, instr), fields(actor_user_id = %token.user_id, image_ver = instr.image_ver))]
pub async fn mark_image_uploaded<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    id: String,
    instr: MarkPageArtworkImageUploadedInstr,
) -> BaseRest<()>
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
    let page_artwork_info = GetPageArtworkInfo { id: &id }.run_on(repo).await?;

    nucl.coord(async move |context| {
        //
        let chapter_info = GetChapterInfoExcluded {
            id: &page_artwork_info.chapter_id,
            incls: &[],
        }
        .step_on(repo, context)
        .await?;

        chapter_complex::ensure_chapter_writable(&chapter_info)?;

        ensure_user_can_write(repo, context, &chapter_info.id, &token.user_id)
            .await?;

        let page_artwork_infos = ListPageArtworkInfos {
            chapter_id: &chapter_info.id,
        }
        .step_on(repo, context)
        .await?;

        if !page_artwork_infos.iter().any(|info| info.id == id) {
            //
            return Err(page_artwork_complex::error(
                ExpectedVariant::Args,
                "error-page-artwork-not-found",
            ));
        }

        let obj_gen = ObjGen {
            id,
            ver: instr.image_ver,
        };

        let marked = MarkObjUploaded::<PageArtworkImage>::new(&obj_gen)
            .step_on(obj_dept, context)
            .await
            .map_err(BaseError::from)?;

        if !marked {
            //
            return Err(page_artwork_complex::error(
                ExpectedVariant::Args,
                "error-stale-page-artwork-upload",
            ));
        }

        TouchComicLastActive {
            id: &chapter_info.comic_id,
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await?;

    accept(())
}
