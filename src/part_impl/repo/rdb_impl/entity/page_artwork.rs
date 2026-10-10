//! Typed composite page rows.

use diesel::{Insertable, Queryable, Selectable};
use time::OffsetDateTime;

use crate::model::read::proj::page_artwork::PageArtworkInfo;
use crate::model::write::page_artwork::PageArtworkEntry;
use crate::part_impl::repo::rdb_impl::numeric::{
    i32_from_usize, usize_from_i32,
};
use crate::part_impl::repo::rdb_impl::schema::t_page_artwork;
use crate::result::{BaseError, BaseRest, accept};

/// Composite page metadata row.
#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = t_page_artwork)]
pub struct PageArtworkRow {
    pub f_id: String,

    pub f_chapter_id: String,
    pub f_index: i32,

    pub f_raw_ident: Option<String>,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl TryFrom<&PageArtworkEntry> for PageArtworkRow {
    type Error = BaseError;

    fn try_from(entry: &PageArtworkEntry) -> BaseRest<Self> {
        //
        let now = OffsetDateTime::now_utc();

        accept(Self {
            f_id: entry.id.clone(),
            f_chapter_id: entry.chapter_id.clone(),
            f_index: i32_from_usize(entry.index, "t_page_artwork.f_index")?,
            f_raw_ident: entry.raw_ident.clone(),
            f_created_at: now,
            f_updated_at: now,
        })
    }
}

impl TryFrom<PageArtworkRow> for PageArtworkInfo {
    type Error = BaseError;

    fn try_from(row: PageArtworkRow) -> BaseRest<Self> {
        //
        accept(Self {
            id: row.f_id,
            chapter_id: row.f_chapter_id,
            index: usize_from_i32(row.f_index, "t_page_artwork.f_index")?,
            raw_ident: row.f_raw_ident,
            created_at: row.f_created_at,
            updated_at: row.f_updated_at,
        })
    }
}
