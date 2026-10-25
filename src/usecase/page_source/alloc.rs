//! Chapter page-manifest and page-image allocation.

// Authoritative chapter manifest transaction.
mod manifest;
// Transactional allocation permission checks.
mod permission;

/// Manifest validation rules.
pub mod validation;

use std::time::Duration;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use poprako_orchestra_extra::prom::oper::Defer;
use poprako_orchestra_extra::prom::task::Task;
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::model::alloc::ObjAlloc;
use poprako_obj_dept::model::slot::ObjSlotSpec;
use poprako_obj_dept::oper::GenObjSlot;

use crate::complex::{
    chapter as chapter_complex, image as image_complex,
    page_source as page_source_complex,
};
use crate::config::image::ImageConfig;
use crate::data::instr::page_source::{
    AllocChapterPageSourcesInstr, AllocPageSourceImageInstr,
};
use crate::data::val::page_source::{
    AllocChapterPageSourcesVal, AllocatedPageSourceVal,
};
use crate::data::view::image::ImageUploadSlotView;
use crate::model::shared::user::UserToken;
use crate::model::write::page_source::PageSourceImageSpec;
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::PageSourceImage;
use crate::part::prom::Prom;
use crate::part::prom::payload::PromPayload;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page_source::{
    GetPageSourceInfo, GetPageSourceInfoExcluded, UpdatePageSourceRawIdents,
};
use crate::part::repo::page_source::PageSourceRepo;
use crate::result::{BaseError, BaseRest};
use crate::shape::write::page_source::PageSourceRawIdentsReplShape;
use crate::usecase::page_source::alloc::manifest::apply_manifest;
use crate::usecase::page_source::alloc::permission::ensure_alloc_perm;
use crate::usecase::page_source::alloc::validation as page_source_alloc_validation_usecase;
use crate::util::next_snowflake_id;
use crate::value::image::{ImageExt, ImageHash, ImageKind, PageSourceImageKey};

/// Allocates the authoritative page manifest and its required image uploads.
#[instrument(
    level = "info",
    skip(nucl, repo, prom, obj_dept, image_config, token),
    fields(
        actor_user_id = %token.user_id,
    ),
)]
pub async fn alloc_chapter_page_sources<N, C, R, P, O>(
    (nucl, repo, prom, obj_dept, image_config): (&N, &R, &P, &O, &ImageConfig),
    token: UserToken,
    instr: AllocChapterPageSourcesInstr,
) -> BaseRest<AllocChapterPageSourcesVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C>
        + ComicRepo<C>
        + AssignmentRepo<C>
        + PageSourceRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
    O: ObjDept<PageSourceImage, C> + Send + Sync,
{
    let AllocChapterPageSourcesInstr { chapter_id, pages } = instr;

    let page_source_specs = pages
        .into_iter()
        .map(PageSourceImageSpec::from)
        .collect::<Vec<_>>();

    let page_source_count =
        page_source_alloc_validation_usecase::validate_page_source_specs(
            image_config,
            &page_source_specs,
            &chapter_id,
            &token.user_id,
        )?;

    let allocs = nucl
        .coord(async move |context| {
            //
            apply_manifest(
                (repo, prom, obj_dept, context),
                &token.user_id,
                &chapter_id,
                &page_source_specs,
                page_source_count,
            )
            .await
        })
        .await?;

    let pages = allocs
        .into_iter()
        .map(|(page_source_id, index, image_hash, ext, obj_alloc)| {
            //
            alloc_val(PageSourceAlloc {
                page_source_id,
                index,
                image_hash,
                ext,
                obj_alloc,
            })
        })
        .collect::<BaseRest<Vec<_>>>()?;

    accept!(AllocChapterPageSourcesVal { pages })
}

/// Allocates an image generation for one page.
#[instrument(
    level = "info",
    skip(nucl, repo, prom, obj_dept, image_config, token),
    fields(
        actor_user_id = %token.user_id,
    ),
)]
pub async fn alloc_image<N, C, R, P, O>(
    (nucl, repo, prom, obj_dept, image_config): (&N, &R, &P, &O, &ImageConfig),
    token: UserToken,
    id: String,
    instr: AllocPageSourceImageInstr,
) -> BaseRest<AllocatedPageSourceVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C>
        + PageSourceRepo<C>
        + AssignmentRepo<C>
        + ComicRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
    O: ObjDept<PageSourceImage, C> + Send + Sync,
{
    page_source_complex::ensure_raw_ident(instr.raw_ident.as_deref())?;

    image_complex::ensure_byte_length(
        image_config,
        instr.new_byte_len,
        ImageKind::PageSourceImage,
    )?;

    let page_source_info = GetPageSourceInfo { id: &id }.run_on(repo).await?;

    let page_source_id = id.clone();

    let image_hash = instr.image_hash.clone();

    let image_ext = instr.ext;

    let (obj_alloc, page_source_index) = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &page_source_info.chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            chapter_complex::ensure_chapter_writable(&chapter_info)?;

            ensure_alloc_perm(repo, context, &token.user_id, &chapter_info.id)
                .await?;

            let locked_page = GetPageSourceInfoExcluded { id: &id }
                .step_on(repo, context)
                .await?;

            let obj_spec = ObjSlotSpec {
                dom: PageSourceImageKey {
                    chapter_id: page_source_info.chapter_id.clone(),
                    page_source_id: id.clone(),
                    ext: instr.ext,
                },
                hash: instr.image_hash.as_bytes(),
                content_type: instr.ext.content_type(),
                byte_len: instr.new_byte_len,
            };

            let obj_alloc = GenObjSlot::<PageSourceImage>::new(&obj_spec)
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)?;

            let raw_ident_repl = PageSourceRawIdentsReplShape {
                idents: vec![(id.as_str(), instr.raw_ident.as_deref())],
            };

            UpdatePageSourceRawIdents {
                repl: &raw_ident_repl,
            }
            .step_on(repo, context)
            .await?;

            if matches!(obj_alloc, ObjAlloc::Upload { .. }) {
                //
                let advance_id = next_snowflake_id();

                let advance_payload = PromPayload::Chapter {
                    payload: ChapterPayload::TryAdvanceRawProvideStage {
                        chapter_id: page_source_info.chapter_id.clone(),
                        actor_user_id: token.user_id.clone(),
                    },
                };

                let advance_task = Task {
                    id: &advance_id,
                    payload: &advance_payload,
                    delay: Some(Duration::from_mins(20)),
                };

                Defer::new(advance_task).step_on(prom, context).await?;
            }

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            accept!((obj_alloc, locked_page.index))
        })
        .await?;

    alloc_val(PageSourceAlloc {
        page_source_id,
        index: page_source_index,
        image_hash,
        ext: image_ext,
        obj_alloc,
    })
}

// Converts one internal allocation into a response view.
struct PageSourceAlloc {
    // Stable page identifier.
    page_source_id: String,

    // Final page position.
    index: usize,

    // Expected content hash.
    image_hash: ImageHash,
    // Expected image suffix.
    ext: ImageExt,

    // Optional new upload capability.
    obj_alloc: ObjAlloc,
}

// Converts one internal allocation into a response view.
fn alloc_val(alloc: PageSourceAlloc) -> BaseRest<AllocatedPageSourceVal> {
    //
    let image_ver = alloc.obj_alloc.key().ver;

    let slot =
        alloc
            .obj_alloc
            .into_slot()
            .map(|obj_alloc| ImageUploadSlotView {
                put_url: obj_alloc.url.to_string(),
                image_ver: obj_alloc.key.ver,
                headers: obj_alloc.headers,
            });

    let index = u32::try_from(alloc.index).map_err(|_| {
        //
        unrecoverable_error! {
            message: "page index is out of range".into();
            page_source_id = %alloc.page_source_id,
            page_source_index = alloc.index,
            "page index is out of range",
        }
    })?;

    accept!(AllocatedPageSourceVal {
        page_source_id: alloc.page_source_id,
        index,
        image_hash: alloc.image_hash,
        ext: alloc.ext,
        image_ver,
        slot,
    })
}
