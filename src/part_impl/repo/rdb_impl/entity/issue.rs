//! Typed Diesel records for current review issues.

#[cfg(test)]
mod tests;

use diesel::{Insertable, Queryable, Selectable};

use crate::model::read::proj::issue::IssueInfo;
use crate::model::shared::issue::IssueRect;
use crate::model::write::issue::IssueEntry;
use crate::part_impl::repo::rdb_impl::numeric::{
    i32_from_usize, usize_from_i32,
};
use crate::part_impl::repo::rdb_impl::schema::t_issue;
use crate::result::{BaseError, BaseRest};

/// Stored issue projection.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_issue)]
pub struct IssueRecord {
    pub f_id: String,

    pub f_page_artwork_id: String,
    pub f_index: i32,

    pub f_variant: String,
    pub f_layer_name: Option<String>,
    pub f_x_coord: Option<f64>,
    pub f_y_coord: Option<f64>,
    pub f_width: Option<f64>,
    pub f_height: Option<f64>,

    pub f_note: String,
}

impl TryFrom<IssueRecord> for IssueInfo {
    type Error = BaseError;

    fn try_from(record: IssueRecord) -> BaseRest<Self> {
        //
        let rect = match (
            record.f_x_coord,
            record.f_y_coord,
            record.f_width,
            record.f_height,
        ) {
            //
            (None, None, None, None) => None,

            (Some(x_coord), Some(y_coord), Some(width), Some(height)) => {
                //
                Some(IssueRect {
                    x_coord,
                    y_coord,
                    width,
                    height,
                })
            }

            _ => {
                //
                return Err(unrecoverable_error! {
                    message: "incomplete stored issue rectangle".into();
                    issue_id = %record.f_id,
                    page_artwork_id = %record.f_page_artwork_id,
                    "incomplete stored issue rectangle",
                });
            }
        };

        accept!(Self {
            id: record.f_id,
            page_artwork_id: record.f_page_artwork_id,
            index: usize_from_i32(record.f_index, "t_issue.f_index")?,
            variant: record.f_variant,
            layer_name: record.f_layer_name,
            rect,
            note: record.f_note,
        })
    }
}

/// Borrowed batch insertion record for issue entries.
#[derive(Insertable)]
#[diesel(table_name = t_issue)]
pub struct IssueItem<'a> {
    pub f_id: &'a str,

    pub f_page_artwork_id: &'a str,
    pub f_index: i32,

    pub f_variant: &'a str,
    pub f_layer_name: Option<&'a str>,
    pub f_x_coord: Option<f64>,
    pub f_y_coord: Option<f64>,
    pub f_width: Option<f64>,
    pub f_height: Option<f64>,

    pub f_note: &'a str,
}

impl<'a> TryFrom<&'a IssueEntry> for IssueItem<'a> {
    type Error = BaseError;

    fn try_from(entry: &'a IssueEntry) -> BaseRest<Self> {
        //
        accept!(Self {
            f_id: &entry.id,
            f_page_artwork_id: &entry.page_artwork_id,
            f_index: i32_from_usize(entry.index, "t_issue.f_index")?,
            f_variant: entry.trimmed_variant(),
            f_layer_name: entry.layer_name.as_deref(),
            f_x_coord: entry.rect.map(|rect| rect.x_coord),
            f_y_coord: entry.rect.map(|rect| rect.y_coord),
            f_width: entry.rect.map(|rect| rect.width),
            f_height: entry.rect.map(|rect| rect.height),
            f_note: &entry.note,
        })
    }
}
