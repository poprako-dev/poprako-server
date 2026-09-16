//! Typed PostgreSQL access to Unit save receipts.

// PostgreSQL tests for concurrent saves and receipt rollback.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
mod tests;

use diesel::{ExpressionMethods as _, OptionalExtension as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Step};
use tracing::instrument;

use crate::model::read::proj::unit_save::UnitSaveInfo;
use crate::part::nucl::Serial;
use crate::part::repo::oper::unit_save::{FindUnitSave, InsertUnitSave};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::schema::t_unit_save;
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::{RdbContext, result};

// Classifies corrupt persisted receipts and serialization errors.
fn receipt_error(error: &serde_json::Error) -> BaseError {
    //
    tracing::error!(sdk_err = ?error, "Unit save receipt serialization failed");

    BaseError::Unrecoverable {
        message: "Invalid Unit save receipt".into(),
    }
}

impl<L> Step<FindUnitSave<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<Serial>,
{
    // Read receipts within the serializable Unit save transaction.
    type Level = Serial;
    // Convert database and receipt decoding failures into application errors.
    type Error = BaseError;

    // Load and decode the receipt matching the complete owner/page/save key.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &FindUnitSave<'_>,
    ) -> BaseRest<Option<UnitSaveInfo>> {
        //
        let row = t_unit_save::table
            .filter(t_unit_save::f_user_id.eq(oper.user_id))
            .filter(t_unit_save::f_page_id.eq(oper.page_id))
            .filter(t_unit_save::f_save_id.eq(oper.save_id))
            .select((
                t_unit_save::f_payload_digest,
                t_unit_save::f_created_unit_ids,
            ))
            .first::<(Vec<u8>, serde_json::Value)>(context.conn())
            .await
            .optional()
            .map_err(result::diesel)?;

        let Some((payload_digest, ids)) = row else {
            return accept(None);
        };

        let created_unit_ids = serde_json::from_value(ids)
            .map_err(|error| receipt_error(&error))?;

        accept(Some(UnitSaveInfo {
            user_id: oper.user_id.into(),
            page_id: oper.page_id.into(),
            save_id: oper.save_id.into(),
            payload_digest,
            created_unit_ids,
        }))
    }
}

impl<L> Step<InsertUnitSave<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<Serial>,
{
    // Persist receipts atomically with the serializable Unit edit batch.
    type Level = Serial;
    // Propagate serialization and database failures to the transaction owner.
    type Error = BaseError;

    // Serialize the created identities and insert the receipt in this transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &InsertUnitSave<'_>,
    ) -> BaseRest<()> {
        //
        let entry = oper.entry;

        let ids = serde_json::to_value(&entry.created_unit_ids)
            .map_err(|error| receipt_error(&error))?;

        diesel::insert_into(t_unit_save::table)
            .values((
                t_unit_save::f_user_id.eq(&entry.user_id),
                t_unit_save::f_page_id.eq(&entry.page_id),
                t_unit_save::f_save_id.eq(&entry.save_id),
                t_unit_save::f_payload_digest.eq(&entry.payload_digest),
                t_unit_save::f_created_unit_ids.eq(ids),
            ))
            .execute(context.conn())
            .await
            .map_err(result::diesel)?;

        accept(())
    }
}
