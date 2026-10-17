//! Preparation of a token-deduplicated Unit save batch.
use std::sync::Arc;

use uuid::Uuid;

use crate::complex::{unit as unit_complex, unit_save as unit_save_complex};
use crate::data::instr::unit::{SavePageUnitEditsInstr, into_unit_edits};
use crate::model::write::unit::UnitEdit;
use crate::result::{BaseRest, accept};
use crate::value::unit_save::UnitSaveCreatedUnitId;

/// Validates the save token and assigns permanent identities to new Units.
pub fn prepare_save(
    instr: SavePageUnitEditsInstr,
    user_id: &str,
) -> BaseRest<(Vec<UnitEdit>, Uuid, Arc<[UnitSaveCreatedUnitId]>)> {
    //
    let save_id = Uuid::parse_str(&instr.save_id)
        .map_err(|_| unit_save_complex::invalid_save())?;

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

    accept((edits, save_id, created_unit_ids.into()))
}
