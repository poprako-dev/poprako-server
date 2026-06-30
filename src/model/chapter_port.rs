//! Internal chapter translation import format models.

use serde::Deserialize;

use crate::model::page_port::PoprakoPageImport;

/// PopRaKo JSON import root.
#[derive(Deserialize)]
pub struct PoprakoProjectImport {
    pub author: String,
    pub title: String,
    pub pages: Vec<PoprakoPageImport>,
}
