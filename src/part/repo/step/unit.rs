//! Step types for unit repository operations.

use poprako_transactional::step::Step;

use crate::model::unit::{UnitCounters, UnitInfo};

/// Step that lists units by page.
pub struct ListInfosByPage<'a> {
    pub page_id: &'a str,
}

impl<'a> Step for ListInfosByPage<'a> {
    type Output = Vec<UnitInfo>;
}

/// Step that replaces the full unit sequence under one page.
pub struct ReplaceInfosByPage<'a> {
    pub page_id: &'a str,
    pub unit_infos: &'a [UnitInfo],
}

impl<'a> Step for ReplaceInfosByPage<'a> {
    type Output = ();
}

/// Step that counts units by page.
pub struct CountByPage<'a> {
    pub page_id: &'a str,
}

impl<'a> Step for CountByPage<'a> {
    type Output = UnitCounters;
}

/// Factory for constructing unit repository [`Step`] values.
pub struct UnitStep;

impl UnitStep {
    /// Constructs a step to list units by page.
    pub fn list_infos_by_page<'a>(page_id: &'a str) -> ListInfosByPage<'a> {
        ListInfosByPage { page_id }
    }

    /// Constructs a step to replace all units under one page.
    pub fn replace_infos_by_page<'a>(
        page_id: &'a str,
        unit_infos: &'a [UnitInfo],
    ) -> ReplaceInfosByPage<'a> {
        ReplaceInfosByPage {
            page_id,
            unit_infos,
        }
    }

    /// Constructs a step to count units under one page.
    pub fn count_by_page<'a>(page_id: &'a str) -> CountByPage<'a> {
        CountByPage { page_id }
    }
}
