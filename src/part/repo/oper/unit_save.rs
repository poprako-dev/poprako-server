//! Transaction-scoped Unit save receipts.
use poprako_orchestra::Oper;

use crate::model::read::proj::unit_save::UnitSaveInfo;
use crate::model::write::unit_save::UnitSaveEntry;

/// Looks up a save after authorization and acquiring the Chapter edit lock.
#[derive(Oper)]
#[allow(
    clippy::struct_field_names,
    reason = "All three identities are distinct parts of the save key."
)]
#[oper(output = Option<UnitSaveInfo>)]
pub struct FindUnitSave<'a> {
    /// Authenticated actor.
    pub user_id: &'a str,
    /// Edited Page.
    pub page_id: &'a str,
    /// Immutable save identity.
    pub save_id: &'a str,
}

/// Records a successful save in the same transaction as its edits.
#[derive(Oper)]
#[oper(output = ())]
pub struct InsertUnitSave<'a> {
    /// Receipt for the committed batch.
    pub entry: &'a UnitSaveEntry,
}
