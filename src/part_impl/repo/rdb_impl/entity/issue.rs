//! Typed Diesel rows for current review issues.

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
use crate::result::{BaseError, BaseRest, accept};

/// Stored issue projection and batch insertion row.
#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = t_issue)]
pub struct IssueRow {
    pub f_id: String,

    pub f_page_artwork_id: String,
    pub f_index: i32,

    pub f_variant: String,
    pub f_layer_path: Option<String>,
    pub f_x_coord: Option<f64>,
    pub f_y_coord: Option<f64>,
    pub f_width: Option<f64>,
    pub f_height: Option<f64>,

    pub f_note: String,
}

impl TryFrom<&IssueEntry> for IssueRow {
    type Error = BaseError;

    fn try_from(entry: &IssueEntry) -> BaseRest<Self> {
        //
        accept(Self {
            f_id: entry.id.clone(),
            f_page_artwork_id: entry.page_artwork_id.clone(),
            f_index: i32_from_usize(entry.index, "t_issue.f_index")?,
            f_variant: entry.variant.clone(),
            f_layer_path: entry.layer_path.clone(),
            f_x_coord: entry.rect.map(|rect| rect.x_coord),
            f_y_coord: entry.rect.map(|rect| rect.y_coord),
            f_width: entry.rect.map(|rect| rect.width),
            f_height: entry.rect.map(|rect| rect.height),
            f_note: entry.note.clone(),
        })
    }
}

impl TryFrom<IssueRow> for IssueInfo {
    type Error = BaseError;

    fn try_from(row: IssueRow) -> BaseRest<Self> {
        //
        let rect =
            match (row.f_x_coord, row.f_y_coord, row.f_width, row.f_height) {
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
                    return Err(BaseError::Unrecoverable {
                        msg: "incomplete stored issue rectangle".into(),
                    });
                }
            };

        accept(Self {
            id: row.f_id,
            page_artwork_id: row.f_page_artwork_id,
            index: usize_from_i32(row.f_index, "t_issue.f_index")?,
            variant: row.f_variant,
            layer_path: row.f_layer_path,
            rect,
            note: row.f_note,
        })
    }
}
