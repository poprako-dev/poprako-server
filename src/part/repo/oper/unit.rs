use poprako_orchestra::Oper;

use crate::model::read::proj::unit::{UnitCountMetrics, UnitInfo, UnitOrder};
use crate::model::write::unit::UnitEdit;

/// Lists unit infos for a page.
#[derive(Oper)]
#[oper(output = Vec<UnitInfo>)]
pub struct ListUnitInfos<'a> {
    /// The page id.
    pub page_id: &'a str,
}

/// Lists unit infos for multiple pages in each page's linked-list order.
#[derive(Oper)]
#[oper(output = Vec<UnitInfo>)]
pub struct ListUnitInfosByPageIds<'a> {
    /// The page ids whose Units should be retrieved.
    pub page_ids: &'a [String],
}

/// Lists requested Unit infos that currently exist.
#[derive(Oper)]
#[oper(output = Vec<UnitInfo>)]
pub struct ListUnitInfosByIds<'a> {
    /// Permanent Unit IDs to retrieve.
    pub ids: &'a [String],
}

/// Lists unit orders for a page.
#[derive(Oper)]
#[oper(output = Vec<UnitOrder>)]
pub struct ListUnitOrders<'a> {
    /// The page id.
    pub page_id: &'a str,
}

/// Applies unit edits (reorder, create, update, delete) for a page.
#[derive(Oper)]
#[oper(output = UnitCountMetrics)]
pub struct ApplyUnitEdits<'a> {
    //
    /// The page id.
    pub page_id: &'a str,

    /// The new unit order for the page.
    pub orders: &'a [UnitOrder],

    /// The batch of unit edits to apply.
    pub edits: &'a [UnitEdit],
}
