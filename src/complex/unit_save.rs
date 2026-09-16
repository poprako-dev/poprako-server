//! Pure validation of replayed Unit save receipts.

use poprako_util::i18n::trl;

use crate::model::read::proj::unit_save::UnitSaveInfo;
use crate::model::shared::unit_save::UnitSaveCreatedUnitId;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Pure rules for immutable Unit save receipts.
pub struct UnitSaveComplex;

impl UnitSaveComplex {
    /// Reports an invalid or reused save identity without exposing payloads.
    pub fn invalid_save() -> BaseError {
        //
        let message = trl("error-invalid-unit-oper");

        tracing::warn!(err_variant = ?ExpectedVariant::Args, err_message = %message,
        "invalid Unit save identity or payload");

        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        }
    }

    /// Returns the original identities only when the replay has identical edits.
    pub fn replay_saved_units(
        receipt: UnitSaveInfo,
        payload_digest: &[u8],
    ) -> BaseRest<Vec<UnitSaveCreatedUnitId>> {
        //
        if receipt.payload_digest != payload_digest {
            return Err(Self::invalid_save());
        }

        accept(receipt.created_unit_ids)
    }
}
