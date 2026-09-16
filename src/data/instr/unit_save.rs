//! Preparation of an immutable Unit save batch.
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

use crate::complex::{unit as unit_complex, unit_save as unit_save_complex};
use crate::data::instr::unit::{SavePageUnitEditsInstr, into_unit_edits};
use crate::model::shared::unit_save::UnitSaveCreatedUnitId;
use crate::model::write::unit::UnitEdit;
use crate::model::write::unit_save::UnitSaveEntry;
use crate::result::{BaseError, BaseRest, accept};

/// Validates the save identity and prepares durable identity pairs and digest.
pub fn prepare_save(
    instr: SavePageUnitEditsInstr,
    user_id: &str,
) -> BaseRest<(Vec<UnitEdit>, UnitSaveEntry)> {
    //
    if Uuid::parse_str(&instr.save_id).is_err() {
        return Err(unit_save_complex::invalid_save());
    }

    let payload = serde_json::to_vec(&instr.edits).map_err(|error| {
        //
        tracing::error!(sdk_err = ?error, "Unit save serialization failed");

        BaseError::Unrecoverable {
            message: "Unit save serialization failed".into(),
        }
    })?;

    let payload_digest = Sha256::digest(&payload).to_vec();

    let mut created_unit_ids = Vec::new();

    let edits = into_unit_edits(instr.edits, user_id, |local_id| {
        //
        let unit_id = unit_complex::gen_id();

        created_unit_ids.push(UnitSaveCreatedUnitId {
            local_id: local_id.into(),
            unit_id: unit_id.clone(),
        });

        unit_id
    })?;

    let entry = UnitSaveEntry {
        user_id: user_id.into(),
        page_id: instr.page_id,
        save_id: instr.save_id,
        payload_digest,
        created_unit_ids,
    };

    accept((edits, entry))
}
