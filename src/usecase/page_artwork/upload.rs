//! Shared composite upload allocation inside caller-owned transactions.

use poprako_orchestra::{Context, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::model::slot::ObjSlotSpec;
use poprako_obj_dept::oper::{GenObjSlot, ListObjMetas};

use crate::complex::image as image_complex;
use crate::config::image::ImageConfig;
use crate::data::val::page_artwork::AllocatedPageArtworkVal;
use crate::data::view::image::ImageUploadSlotView;
use crate::model::write::page_artwork::PageArtworkEntry;
use crate::part::obj_dept::PageArtworkImage;
use crate::result::{BaseError, BaseRest, accept};
use crate::value::image::{ImageExt, ImageHash, ImageKind};
use crate::value::page_artwork::PageArtworkImageKey;

/// Allocates one image or reuses uploaded identical content on this exact page.
#[instrument(level = "info", skip_all)]
pub async fn alloc_image_slot<C, O>(
    (obj_dept, image_config): (&O, &ImageConfig),
    context: &mut C,
    page_artwork_entry: &PageArtworkEntry,
    (hash, ext, byte_len): (ImageHash, ImageExt, Option<u64>),
) -> BaseRest<AllocatedPageArtworkVal>
where
    C: Context + Send,
    O: ObjDept<PageArtworkImage, C> + Sync,
{
    if let Some(byte_len) = byte_len {
        //
        image_complex::ensure_byte_length(
            image_config,
            byte_len,
            ImageKind::PageImage,
        )?;
    }

    let metas = ListObjMetas::<PageArtworkImage>::new(&[page_artwork_entry
        .id
        .as_str()])
    .step_on(obj_dept, context)
    .await
    .map_err(BaseError::from)?;

    let avail_meta = metas.get(&page_artwork_entry.id).filter(|meta| {
        //
        meta.is_avail
            && meta.hash == hash.as_bytes()
            && meta.ext == ext.suffix()
    });

    if let Some(meta) = avail_meta {
        //
        return accept(AllocatedPageArtworkVal {
            page_artwork_id: page_artwork_entry.id.clone(),
            index: page_artwork_entry.index,
            image_ver: meta.key.ver,
            image_hash: hash,
            ext,
            slot: None,
        });
    }

    let byte_len = byte_len.ok_or_else(|| {
        //
        image_complex::invalid_byte_length_rejection(
            image_config,
            0,
            ImageKind::PageImage,
        )
    })?;

    let spec = ObjSlotSpec {
        dom: PageArtworkImageKey {
            chapter_id: page_artwork_entry.chapter_id.clone(),
            page_artwork_id: page_artwork_entry.id.clone(),
            ext,
        },
        hash: hash.as_bytes(),
        content_type: ext.content_type(),
        byte_len,
    };

    let slot = GenObjSlot::<PageArtworkImage>::new(&spec)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    let metas = ListObjMetas::<PageArtworkImage>::new(&[page_artwork_entry
        .id
        .as_str()])
    .step_on(obj_dept, context)
    .await
    .map_err(BaseError::from)?;

    let meta = metas.get(&page_artwork_entry.id).ok_or_else(|| {
        //
        tracing::error!("allocated composite image metadata is missing");

        BaseError::Unrecoverable {
            msg: "allocated composite image metadata is missing".into(),
        }
    })?;

    accept(AllocatedPageArtworkVal {
        page_artwork_id: page_artwork_entry.id.clone(),
        index: page_artwork_entry.index,
        image_ver: meta.key.ver,
        image_hash: hash,
        ext,
        slot: slot.map(|slot| ImageUploadSlotView {
            image_ver: slot.key.ver,
            put_url: slot.url.to_string(),
            headers: slot.headers,
        }),
    })
}
