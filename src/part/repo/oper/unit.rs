use poprako_orchestra::Oper;

use crate::model::read::proj::unit::{UnitCountMetrics, UnitInfo, UnitOrder};
use crate::model::write::unit::UnitEdit;
use crate::value::unit::UnitTextPart;

/// Lists unit infos for multiple pages in each page's linked-list order.
#[derive(Oper)]
#[oper(output = Vec<UnitInfo>)]
pub struct ListUnitInfosByPageIds<'a> {
    /// The page ids whose Units should be retrieved.
    pub page_ids: &'a [&'a str],
}

/// Lists requested Unit infos that currently exist.
#[derive(Oper)]
#[oper(output = Vec<UnitInfo>)]
pub struct ListUnitInfosByIds<'a> {
    /// Permanent Unit IDs to retrieve.
    pub ids: &'a [&'a str],
}

/// Searches visible Unit IDs within one Chapter.
#[derive(Oper)]
#[oper(output = Vec<String>)]
pub struct SearchChapterUnitIds<'a> {
    //
    /// Chapter whose Units should be searched.
    pub chapter_id: &'a str,

    /// Unit text field selected for matching.
    pub part: UnitTextPart,
    /// Normalized literal phrase to find.
    pub phrase: &'a str,

    /// Maximum candidate rows to fetch for overflow detection.
    pub fetch_count: usize,
}

/// Loads selected Unit infos in Chapter Page and linked-list order.
#[derive(Oper)]
#[oper(output = Vec<UnitInfo>)]
pub struct ListUnitInfosInChapterOrder<'a> {
    /// Permanent Unit IDs to retrieve and order.
    pub ids: &'a [&'a str],
}

/// Lists unit orders for a page.
#[derive(Oper)]
#[oper(output = Vec<UnitOrder>)]
pub struct ListUnitOrders<'a> {
    /// The page id.
    pub page_id: &'a str,
}

/// Applies unit edits (reorder, create, update, delete) for a page.
///
/// The caller must hold the owning Chapter row lock and must supply `orders`
/// from the same transaction snapshot.
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
