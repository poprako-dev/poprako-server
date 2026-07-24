//! Internal chapter translation import format models.

use serde::Deserialize;

use crate::model::page_port::PoprakoPageImport;

/// PopRaKo JSON import root.
#[derive(Deserialize)]
pub struct ChapterPoprakoProjectImport {
    //
    /// Imported author name from the source project metadata.
    pub author: String,
    /// Imported comic title from the source project metadata.
    pub title: String,
    /// Ordered list of imported page data for this chapter.
    pub pages: Vec<PoprakoPageImport>,
}
