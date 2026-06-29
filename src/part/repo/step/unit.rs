//! Step types for unit repository opers.

use poprako_transactional::step::Step;

use crate::model::unit::{UnitCounters, UnitIndex, UnitIndexUpdate, UnitInfo, UnitOper};

/// Step that lists units by page.
pub struct ListInfosByPage<'a> {
    pub page_id: &'a str,
}

impl<'a> Step for ListInfosByPage<'a> {
    type Output = Vec<UnitInfo>;
}

/// Step that creates one unit row.
pub struct CreateInfo<'a> {
    pub page_id: &'a str,
    pub oper: &'a UnitOper,
}

impl<'a> Step for CreateInfo<'a> {
    type Output = ();
}

/// Step that saves one unit row by upsert.
pub struct SaveInfo<'a> {
    pub page_id: &'a str,
    pub oper: &'a UnitOper,
}

impl<'a> Step for SaveInfo<'a> {
    type Output = ();
}

/// Step that deletes one unit by page and unit id.
pub struct DeleteByPageIdAndId<'a> {
    pub page_id: &'a str,
    pub id: &'a str,
}

impl<'a> Step for DeleteByPageIdAndId<'a> {
    type Output = ();
}

/// Step that lists persisted unit indexes by page.
pub struct ListIndexesByPage<'a> {
    pub page_id: &'a str,
}

impl<'a> Step for ListIndexesByPage<'a> {
    type Output = Vec<UnitIndex>;
}

/// Step that updates changed indexes for one page.
pub struct UpdateIndexesByPage<'a> {
    pub page_id: &'a str,
    pub updates: &'a [UnitIndexUpdate],
}

impl<'a> Step for UpdateIndexesByPage<'a> {
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

    /// Constructs a step to create one unit row.
    pub fn create_info<'a>(page_id: &'a str, oper: &'a UnitOper) -> CreateInfo<'a> {
        CreateInfo { page_id, oper }
    }

    /// Constructs a step to save one unit row by upsert.
    pub fn save_info<'a>(page_id: &'a str, oper: &'a UnitOper) -> SaveInfo<'a> {
        SaveInfo { page_id, oper }
    }

    /// Constructs a step to delete one unit by page and unit id.
    pub fn delete_by_page_id_and_id<'a>(page_id: &'a str, id: &'a str) -> DeleteByPageIdAndId<'a> {
        DeleteByPageIdAndId { page_id, id }
    }

    /// Constructs a step to list indexes by page.
    pub fn list_indexes_by_page<'a>(page_id: &'a str) -> ListIndexesByPage<'a> {
        ListIndexesByPage { page_id }
    }

    /// Constructs a step to update changed indexes by page.
    pub fn update_indexes_by_page<'a>(
        page_id: &'a str,
        updates: &'a [UnitIndexUpdate],
    ) -> UpdateIndexesByPage<'a> {
        UpdateIndexesByPage { page_id, updates }
    }

    /// Constructs a step to count units under one page.
    pub fn count_by_page<'a>(page_id: &'a str) -> CountByPage<'a> {
        CountByPage { page_id }
    }
}
