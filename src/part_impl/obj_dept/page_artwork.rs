//! Canonical composite image keys.

use poprako_obj_dept::key::KeyMap;
use poprako_obj_dept::rest::{ObjDeptError, ObjDeptRest};

use crate::part::obj_dept::PageArtworkImage;
use crate::value::image::ImageExt;
use crate::value::page_artwork::PageArtworkImageKey;

impl KeyMap for PageArtworkImage {
    // Business composite image key.
    type Dom = PageArtworkImageKey;
    // Physical object key.
    type Img = String;

    // Builds the canonical physical key for one generation.
    fn forward(value: &Self::Dom, ver: u32) -> Self::Img {
        //
        format!(
            "page_artwork/chapter_{}/{}-{}.{}",
            value.chapter_id,
            value.page_artwork_id,
            ver,
            value.ext.suffix()
        )
    }

    // Parses and validates a canonical composite image key.
    fn reverse(value: &Self::Img) -> ObjDeptRest<(Self::Dom, u32)> {
        //
        let parse = || {
            //
            let path = value.strip_prefix("page_artwork/chapter_")?;

            let (chapter_id, filename) = path.split_once('/')?;

            let (stem, ext) = filename.rsplit_once('.')?;

            let (page_artwork_id, ver) = stem.rsplit_once('-')?;

            let dom = PageArtworkImageKey {
                chapter_id: chapter_id.into(),
                page_artwork_id: page_artwork_id.into(),
                ext: ImageExt::parse(ext)?,
            };

            let ver = ver.parse().ok()?;

            (!chapter_id.is_empty()
                && !page_artwork_id.is_empty()
                && Self::forward(&dom, ver) == *value)
                .then_some((dom, ver))
        };

        parse().ok_or_else(|| {
            ObjDeptError::invalid("invalid composite image physical key".into())
        })
    }

    // Returns the stable composite page identity.
    fn id(value: &Self::Dom) -> &str {
        &value.page_artwork_id
    }

    // Returns the composite image suffix.
    fn ext(value: &Self::Dom) -> &str {
        value.ext.suffix()
    }
}
