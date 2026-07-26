use super::*;

// create_unit(create_unit)(positive): a new id is inserted once.
// create_unit(create_unit)(negative): an existing id is rejected without mutation.
// save_unit(save_unit)(positive): a missing id is created and an existing id is updated.

fn payload(text: &str, proofread: bool) -> UnitBody {
    UnitBody {
        is_bubble: true,
        is_proofread: proofread,
        x_coord: 1.0,
        y_coord: 2.0,
        translated_text: Some(text.into()),
        last_translator_id: Some("user-1".into()),
        proofread_text: None,
        last_proofreader_id: None,
    }
}

#[test]
fn create_unit_inserts_once_and_rejects_duplicate() {
    //
    let mut state = MockState::default();

    let unit_payload = payload("translated", false);

    let first_result =
        create_unit(&mut state, "page-1", "unit-1", &unit_payload);

    assert!(first_result.is_ok());

    let duplicate_result =
        create_unit(&mut state, "page-1", "unit-1", &unit_payload);

    assert!(duplicate_result.is_err());

    assert_eq!(state.units.len(), 1);

    assert_eq!(
        state.units[0].translated_text.as_deref(),
        Some("translated")
    );
}

#[test]
fn save_unit_creates_missing_and_updates_existing() {
    //
    let mut state = MockState::default();

    let initial_payload = payload("initial", false);

    save_unit(&mut state, "page-1", "unit-1", &initial_payload)
        .ok()
        .unwrap();

    let updated_payload = payload("updated", true);

    save_unit(&mut state, "page-1", "unit-1", &updated_payload)
        .ok()
        .unwrap();

    assert_eq!(state.units.len(), 1);

    assert_eq!(state.units[0].translated_text.as_deref(), Some("updated"));

    assert!(state.units[0].is_proofread);
}
