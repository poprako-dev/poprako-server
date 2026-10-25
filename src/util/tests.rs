// next_snowflake_u64 (positive): generated snowflake values should be monotonic and fit into u64.
// next_snowflake_id (positive): generated snowflake strings should be base62 and round-trip via base62::decode.
use super::*;

#[test]
fn next_snowflake_u64_generates_monotonic_ids() {
    //
    let first_id = next_snowflake_u64();

    let second_id = next_snowflake_u64();

    assert!(second_id > first_id);
}

#[test]
fn next_snowflake_id_generates_base62_string() {
    //
    let snowflake_id = next_snowflake_id();

    let parsed = base62::decode(&snowflake_id);

    assert!(parsed.is_ok(), "id {snowflake_id:?} is not valid base62");
}

// business_ids(next_business_id)(positive): every registered namespace produces distinct canonical IDs.
#[test]
fn business_ids_use_the_exact_registered_namespace() {
    use crate::value::business_id::BusinessIdKind;

    let kinds = [
        BusinessIdKind::User,
        BusinessIdKind::Team,
        BusinessIdKind::Member,
        BusinessIdKind::MemberInvitation,
        BusinessIdKind::Workset,
        BusinessIdKind::Comic,
        BusinessIdKind::Chapter,
        BusinessIdKind::PageSource,
        BusinessIdKind::PageArtwork,
        BusinessIdKind::Unit,
        BusinessIdKind::Issue,
        BusinessIdKind::Assignment,
        BusinessIdKind::AssignmentInvitation,
        BusinessIdKind::Announcement,
        BusinessIdKind::Comment,
        BusinessIdKind::ComicArchive,
        BusinessIdKind::Termbase,
        BusinessIdKind::Term,
        BusinessIdKind::SystemMail,
        BusinessIdKind::ChapterWorkflowRecord,
    ];

    let mut ids = std::collections::HashSet::new();

    for kind in kinds {
        let id = next_business_id(kind);

        assert!(kind.is_valid_id(&id));

        assert!(id.starts_with(&format!("{}--", kind.prefix())));

        assert!(ids.insert(id.clone()));

        for other in kinds.into_iter().filter(|other| *other != kind) {
            assert!(!other.is_valid_id(&id));
        }
    }
}

// business_ids(is_valid_id)(negative): old separators, malformed suffixes, and overflowing numbers are rejected.
#[test]
fn malformed_business_ids_are_rejected() {
    use crate::value::business_id::BusinessIdKind;

    for id in [
        "page_source-00000000001",
        "page_source__00000000001",
        "page--00000000001",
        "page_source--1",
        "page_source--000000000001",
        "page_source--0000000000/",
        "page_source--zzzzzzzzzzz",
        "page_source--00000000001/",
        "page_source--00000000001--",
    ] {
        assert!(!BusinessIdKind::PageSource.is_valid_id(id), "{id}");
    }
}
