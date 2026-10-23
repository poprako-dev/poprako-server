//! Typed composite page records.

use diesel::{Insertable, Queryable, Selectable};
use time::OffsetDateTime;

use crate::model::read::proj::page_artwork::PageArtworkInfo;
use crate::model::write::page_artwork::PageArtworkEntry;
use crate::part_impl::repo::rdb_impl::numeric::{
    i32_from_usize, usize_from_i32,
};
use crate::part_impl::repo::rdb_impl::schema::t_page_artwork;
use crate::result::{BaseError, BaseRest, accept};

/// Composite page metadata record.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_page_artwork)]
pub struct PageArtworkRecord {
    pub f_id: String,

    pub f_chapter_id: String,
    pub f_index: i32,

    pub f_raw_ident: Option<String>,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl TryFrom<PageArtworkRecord> for PageArtworkInfo {
    type Error = BaseError;

    fn try_from(record: PageArtworkRecord) -> BaseRest<Self> {
        //
        accept(Self {
            id: record.f_id,
            chapter_id: record.f_chapter_id,
            index: usize_from_i32(record.f_index, "t_page_artwork.f_index")?,
            raw_ident: record.f_raw_ident,
            created_at: record.f_created_at,
            updated_at: record.f_updated_at,
        })
    }
}

/// Borrowed batch insertion item for page artwork entries.
#[derive(Insertable)]
#[diesel(table_name = t_page_artwork)]
pub struct PageArtworkItem<'a> {
    pub f_id: &'a str,

    pub f_chapter_id: &'a str,
    pub f_index: i32,

    pub f_raw_ident: Option<&'a str>,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl<'a> TryFrom<&'a PageArtworkEntry> for PageArtworkItem<'a> {
    type Error = BaseError;

    fn try_from(entry: &'a PageArtworkEntry) -> BaseRest<Self> {
        //
        let now = OffsetDateTime::now_utc();

        accept(Self {
            f_id: &entry.id,
            f_chapter_id: &entry.chapter_id,
            f_index: i32_from_usize(entry.index, "t_page_artwork.f_index")?,
            f_raw_ident: entry.raw_ident.as_deref(),
            f_created_at: now,
            f_updated_at: now,
        })
    }
}
