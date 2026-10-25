//! One-way physical mapping for validated artwork identities.

use poprako_obj_dept::key::{ObjKind, ObjPath};

use crate::part::obj_dept::ChapterArtwork;
use crate::value::artwork::{ArtworkExt, ChapterArtworkKey};
use crate::value::business_id::ChapterId;

impl ObjKind for ChapterArtwork {
    // Validated identity of the object owner.
    type Id = ChapterId;
    // Validated file extension.
    type Ext = ArtworkExt;
    // Validated inputs to physical key construction.
    type Dom = ChapterArtworkKey;

    // Returns the validated owner identity.
    fn id(value: &Self::Dom) -> &Self::Id {
        &value.chapter_id
    }

    // Returns the validated file extension.
    fn ext(value: &Self::Dom) -> &Self::Ext {
        &value.ext
    }

    // Constructs the physical location for one allocated generation.
    fn forward(value: &Self::Dom, ver: u32) -> ObjPath {
        //
        ObjPath::new(format!(
            "chapter_artwork/{}/{}.{}",
            value.chapter_id.as_str(),
            ver,
            value.ext.as_str()
        ))
    }
}
