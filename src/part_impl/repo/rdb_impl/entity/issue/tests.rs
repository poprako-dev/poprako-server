#![allow(
    clippy::unwrap_used,
    reason = "Conversion fixtures and assertions fail immediately on violated invariants"
)]

use super::{IssueEntryRow, IssueRow};

use crate::model::read::proj::issue::IssueInfo;
use crate::model::write::issue::IssueEntry;
use crate::result::BaseError;

// Builds an issue with an unsigned domain index for storage conversion tests.
fn entry(index: usize) -> IssueEntry {
    IssueEntry {
        id: "issue".into(),
        page_artwork_id: "artwork".into(),
        index,
        variant: "text".into(),
        layer_path: None,
        rect: None,
        note: String::new(),
    }
}

// Builds a stored row independently from the borrowed insertion conversion.
fn stored_row(index: i32) -> IssueRow {
    IssueRow {
        f_id: "issue".into(),
        f_page_artwork_id: "artwork".into(),
        f_index: index,
        f_variant: "text".into(),
        f_layer_path: None,
        f_x_coord: None,
        f_y_coord: None,
        f_width: None,
        f_height: None,
        f_note: String::new(),
    }
}

// issue_index_roundtrips(IssueRow)(positive): zero and the largest database index retain their unsigned domain values.
#[test]
fn issue_index_roundtrips() {
    for index in [0, usize::try_from(i32::MAX).unwrap()] {
        let issue_entry = entry(index);

        let row = IssueEntryRow::try_from(&issue_entry).unwrap();

        let issue_info = IssueInfo::try_from(stored_row(row.f_index)).unwrap();

        assert_eq!(issue_info.index, index);
    }
}

// issue_index_overflow_is_rejected(IssueRow)(negative): a domain index beyond INTEGER range cannot be persisted.
#[test]
fn issue_index_overflow_is_rejected() {
    let issue_entry = entry(usize::try_from(i32::MAX).unwrap() + 1);

    assert!(matches!(
        IssueEntryRow::try_from(&issue_entry),
        Err(BaseError::Unrecoverable { .. })
    ));
}

// negative_stored_issue_index_is_rejected(IssueInfo)(negative): corrupt signed storage never becomes an unsigned domain index.
#[test]
fn negative_stored_issue_index_is_rejected() {
    let row = stored_row(-1);

    assert!(matches!(
        IssueInfo::try_from(row),
        Err(BaseError::Unrecoverable { .. })
    ));
}
