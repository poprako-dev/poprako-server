//! One-way physical mapping for validated finished-page identities.

use poprako_obj_dept::key::{ObjKind, ObjPath};

use crate::part::obj_dept::PageArtworkImage;
use crate::value::business_id::PageArtworkId;
use crate::value::image::ImageExt;
use crate::value::page_artwork::PageArtworkImageKey;

impl ObjKind for PageArtworkImage {
    // Validated identity of the object owner.
    type Id = PageArtworkId;
    // Validated file extension.
    type Ext = ImageExt;
    // Validated inputs to physical key construction.
    type Dom = PageArtworkImageKey;

    // Returns the validated owner identity.
    fn id(value: &Self::Dom) -> &Self::Id {
        &value.page_artwork_id
    }

    // Returns the validated file extension.
    fn ext(value: &Self::Dom) -> &Self::Ext {
        &value.ext
    }

    // Constructs the physical location for one allocated generation.
    fn forward(value: &Self::Dom, ver: u32) -> ObjPath {
        //
        ObjPath::new(format!(
            "page_artwork/{}/{}/{}.{}",
            value.chapter_id.as_str(),
            value.page_artwork_id.as_str(),
            ver,
            value.ext.suffix()
        ))
    }
}
