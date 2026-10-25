#![allow(
    clippy::unwrap_used,
    reason = "Validated fixture construction fails immediately"
)]

//! Validated identities and canonical locations from persisted construction data.

use poprako_obj_dept::key::{ObjId, ObjKey, ObjKind};
use poprako_obj_dept::model::record::{ObjKeyRecord, ObjMetaRecord};
use poprako_obj_dept::rest::ObjDeptError;

use crate::part::obj_dept::{
    ChapterArtwork, ComicCover, PageArtworkImage, PageSourceImage, TeamAvatar,
    UserAvatar,
};
use crate::value::artwork::ArtworkExt;

// identity(ObjKind)(positive,negative): each associated identity rejects every other namespace and malformed encoding.
fn assert_identity<K: ObjKind>(valid: &str) {
    let identity = K::Id::from_text(valid).unwrap();

    assert_eq!(identity.as_str(), valid);

    let buffer = identity.as_str().as_ptr();

    let text: String = identity.into();

    assert_eq!(text.as_ptr(), buffer);

    for other in [
        "user--00000000001",
        "team--00000000001",
        "comic--00000000001",
        "chapter--00000000001",
        "page_source--00000000001",
        "page_artwork--00000000001",
    ] {
        if other == valid {
            continue;
        }

        assert!(K::Id::from_text(other).is_err());
    }

    for malformed in ["", "page_source_1", "page_source--0000000000!1"] {
        assert!(K::Id::from_text(malformed).is_err());
    }
}

#[test]
fn identities_validate_the_registered_business_namespace() {
    assert_identity::<UserAvatar>("user--00000000001");

    assert_identity::<TeamAvatar>("team--00000000001");

    assert_identity::<ComicCover>("comic--00000000001");

    assert_identity::<ChapterArtwork>("chapter--00000000001");

    assert_identity::<PageSourceImage>("page_source--00000000001");

    assert_identity::<PageArtworkImage>("page_artwork--00000000001");
}

// restore(ObjKey)(positive): validated construction fields determine one canonical location.
#[test]
fn persisted_domains_generate_canonical_locations() -> Result<(), ObjDeptError>
{
    let record = source_record();

    let key = ObjKey::<PageSourceImage>::from_record(record.clone())?;

    assert_eq!(
        key.image().as_str(),
        "page_source/chapter--00000000001/page_source--00000000001/7.png"
    );

    assert_eq!(key.to_record(), record);

    let hash = vec![17; 32];

    let hash_buffer = hash.as_ptr();

    let metadata = ObjMetaRecord {
        key: record,
        is_avail: true,
        hash,
        ext: "png".into(),
    }
    .decode::<PageSourceImage>()?;

    assert_eq!(metadata.hash.as_ptr(), hash_buffer);

    Ok(())
}

// restore(ObjKey)(negative): missing, extra, cross-kind, and malformed fields cannot enter typed operations.
#[test]
fn persisted_domains_reject_invalid_and_inconsistent_fields() {
    for (field, value) in [
        ("chapter_id", serde_json::json!("user--00000000001")),
        (
            "page_source_id",
            serde_json::json!("page_source--00000000002"),
        ),
        ("ext", serde_json::json!("unsupported")),
        ("image", serde_json::json!("archive-v1/blob.png")),
        ("chapter_id", serde_json::Value::Null),
    ] {
        let mut record = source_record();

        record
            .domain
            .as_object_mut()
            .unwrap()
            .insert(field.into(), value);

        assert!(matches!(
            ObjKey::<PageSourceImage>::from_record(record),
            Err(ObjDeptError::Unrecoverable { .. })
        ));
    }

    let mut record = source_record();

    record.id = "user--00000000001".into();

    assert!(ObjKey::<PageSourceImage>::from_record(record).is_err());

    let metadata = ObjMetaRecord {
        key: source_record(),
        is_avail: true,
        hash: vec![17; 32],
        ext: "jpg".into(),
    };

    assert!(metadata.decode::<PageSourceImage>().is_err());

    let legacy = serde_json::json!({
        "id": "page_source--00000000001",
        "ver": 7,
        "image": "archive-v1/blob.png",
    });

    assert!(serde_json::from_value::<ObjKeyRecord>(legacy).is_err());
}

fn source_record() -> ObjKeyRecord {
    ObjKeyRecord {
        id: "page_source--00000000001".into(),
        ver: 7,
        domain: serde_json::json!({
            "chapter_id": "chapter--00000000001",
            "page_source_id": "page_source--00000000001",
            "ext": "png",
        }),
    }
}

// suffix(ArtworkExt)(positive,negative): arbitrary safe file suffixes are validated independently of physical keys.
#[test]
fn artwork_suffix_validation_preserves_case_and_rejects_path_fragments() {
    for suffix in ["PSD", "zip", "archive_1-raw"] {
        assert_eq!(
            ArtworkExt::new(suffix).map(|ext| ext.as_str().to_owned()),
            Ok(suffix.to_owned())
        );
    }

    for suffix in ["", "../png", "a/b", "a.png", "\nzip"] {
        assert!(ArtworkExt::new(suffix).is_err());
    }
}
