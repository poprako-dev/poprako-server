//! Step types for unit repository opers.

use poprako_transactional::step::Step;
use poprako_util::page::Page;

use crate::model::unit::{
    UnitCounters, UnitIndex, UnitIndexUpdate, UnitInfo, UnitOper,
};

/// Step that lists units by page ID.
pub struct ListInfosByPageId<'a> {
    pub page_id: &'a str,

    pub page: Page,
}

impl<'a> Step for ListInfosByPageId<'a> {
    type Output = Vec<UnitInfo>;
}

/// Step that lists all units by page ID (no pagination).
pub struct ListAllInfosByPageId<'a> {
    pub page_id: &'a str,
}

impl<'a> Step for ListAllInfosByPageId<'a> {
    type Output = Vec<UnitInfo>;
}

/// Step that saves one unit row by upsert.
pub struct SaveInfo<'a> {
    pub page_id: &'a str,
    pub oper: &'a UnitOper,
}

impl<'a> Step for SaveInfo<'a> {
    type Output = ();
}

/// Step that deletes one unit by ID, scoped to a page.
pub struct DeleteByIdInPage<'a> {
    pub page_id: &'a str,
    pub id: &'a str,
}

impl<'a> Step for DeleteByIdInPage<'a> {
    type Output = ();
}

/// Step that lists persisted unit indexes by page ID.
pub struct ListIndexesByPageId<'a> {
    pub page_id: &'a str,
}

impl<'a> Step for ListIndexesByPageId<'a> {
    type Output = Vec<UnitIndex>;
}

/// Step that updates changed indexes for one page ID.
pub struct UpdateIndexesByPageId<'a> {
    pub page_id: &'a str,
    pub updates: &'a [UnitIndexUpdate],
}

impl<'a> Step for UpdateIndexesByPageId<'a> {
    type Output = ();
}

/// Step that counts units by page ID.
pub struct CountByPageId<'a> {
    pub page_id: &'a str,
}

impl<'a> Step for CountByPageId<'a> {
    type Output = UnitCounters;
}

/// Factory for constructing unit repository [`Step`] values.
pub struct UnitStep;

impl UnitStep {
    /// Constructs a step to list units by page ID.
    pub fn list_infos_by_page_id<'a>(
        page_id: &'a str,
        page: Page,
    ) -> ListInfosByPageId<'a> {
        ListInfosByPageId { page_id, page }
    }

    /// Constructs a step to list all units by page ID (no pagination).
    pub fn list_all_infos_by_page_id<'a>(
        page_id: &'a str,
    ) -> ListAllInfosByPageId<'a> {
        ListAllInfosByPageId { page_id }
    }

    /// Constructs a step to save one unit row by upsert.
    pub fn save_info<'a>(page_id: &'a str, oper: &'a UnitOper) -> SaveInfo<'a> {
        SaveInfo { page_id, oper }
    }

    /// Constructs a step to delete one unit by ID, scoped to a page.
    pub fn delete_by_id_in_page<'a>(
        page_id: &'a str,
        id: &'a str,
    ) -> DeleteByIdInPage<'a> {
        DeleteByIdInPage { page_id, id }
    }

    /// Constructs a step to list indexes by page ID.
    pub fn list_indexes_by_page_id<'a>(
        page_id: &'a str,
    ) -> ListIndexesByPageId<'a> {
        ListIndexesByPageId { page_id }
    }

    /// Constructs a step to update changed indexes by page ID.
    pub fn update_indexes_by_page_id<'a>(
        page_id: &'a str,
        updates: &'a [UnitIndexUpdate],
    ) -> UpdateIndexesByPageId<'a> {
        UpdateIndexesByPageId { page_id, updates }
    }

    /// Constructs a step to count units under one page ID.
    pub fn count_by_page_id<'a>(page_id: &'a str) -> CountByPageId<'a> {
        CountByPageId { page_id }
    }
}
