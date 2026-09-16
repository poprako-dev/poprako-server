//! Transactional in-memory Unit save receipts.
use poprako_orchestra::Step;
use tracing::instrument;

use crate::model::read::proj::unit_save::UnitSaveInfo;
use crate::part::nucl::Serial;
use crate::part::repo::oper::unit_save::{FindUnitSave, InsertUnitSave};
use crate::part_impl::repo::mock_impl::{Mock, MockContext};
use crate::result::{BaseError, BaseRest, accept};

impl Step<FindUnitSave<'_>, MockContext> for Mock {
    // Receipt lookups participate in the serializable save transaction.
    type Level = Serial;
    // Propagate application errors through the repository contract.
    type Error = BaseError;

    // Find the receipt owned by this actor, page, and save identity.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &FindUnitSave<'_>,
    ) -> BaseRest<Option<UnitSaveInfo>> {
        //
        accept(
            context
                .state
                .unit_saves
                .iter()
                .find(|save| {
                    //
                    save.user_id == oper.user_id
                        && save.page_id == oper.page_id
                        && save.save_id == oper.save_id
                })
                .cloned(),
        )
    }
}

impl Step<InsertUnitSave<'_>, MockContext> for Mock {
    // Receipt insertion must commit or roll back with the Unit edits.
    type Level = Serial;
    // Use the same application error type as the save transaction.
    type Error = BaseError;

    // Append the receipt to transactional mock state for rollback support.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &InsertUnitSave<'_>,
    ) -> BaseRest<()> {
        //
        let entry = oper.entry;

        context.state.unit_saves.push(UnitSaveInfo {
            user_id: entry.user_id.clone(),
            page_id: entry.page_id.clone(),
            save_id: entry.save_id.clone(),
            payload_digest: entry.payload_digest.clone(),
            created_unit_ids: entry.created_unit_ids.clone(),
        });

        accept(())
    }
}
