use url::Url;

use super::*;

#[test]
fn single_use_url_moves_generated_bytes_and_consumes_its_entry() {
    //
    let origin = Url::parse("https://obj.test/origin").unwrap();

    let origin_pointer = origin.as_str().as_ptr();

    let thumbnail = Url::parse("https://obj.test/thumbnail").unwrap();

    let thumbnail_pointer = thumbnail.as_str().as_ptr();

    let urls = ObjUrls {
        origin_url: Some(origin),
        optimized_url: None,
        thumbnail_url: Some(thumbnail),
    };

    let mut batch = ObjUrlBatch::from_urls(
        HashMap::from([(String::from("object-1"), urls)]),
        &HashMap::from([("object-1", 1)]),
    );

    let (origin, thumbnail) = batch.take("object-1").unwrap();

    assert_eq!(origin.unwrap().as_ptr(), origin_pointer);

    assert_eq!(thumbnail.unwrap().as_ptr(), thumbnail_pointer);

    assert!(!batch.urls.contains_key("object-1"));

    assert!(batch.take("object-1").is_none());
}

#[test]
fn repeated_url_shares_generated_bytes_until_final_consumption() {
    //
    let origin = Url::parse("https://obj.test/repeated").unwrap();

    let pointer = origin.as_str().as_ptr();

    let urls = ObjUrls {
        origin_url: Some(origin),
        optimized_url: None,
        thumbnail_url: None,
    };

    let mut batch = ObjUrlBatch::from_urls(
        HashMap::from([(String::from("object-1"), urls)]),
        &HashMap::from([("object-1", 3)]),
    );

    let (first, _) = batch.take("object-1").unwrap();

    let (second, _) = batch.take("object-1").unwrap();

    let Some(CachedObjUrls::Repeated {
        remaining, origin, ..
    }) = batch.urls.get("object-1")
    else {
        panic!("the final consumer must retain the shared cache entry");
    };

    assert_eq!(*remaining, 1);

    assert_eq!(Arc::strong_count(origin.as_ref().unwrap()), 3);

    let (last, _) = batch.take("object-1").unwrap();

    assert!(!batch.urls.contains_key("object-1"));

    for view in [first, second, last] {
        assert_eq!(view.unwrap().as_ptr(), pointer);
    }
}

#[test]
fn existing_metadata_with_absent_urls_remains_distinct_from_missing_metadata() {
    //
    let urls = ObjUrls {
        origin_url: None,
        optimized_url: None,
        thumbnail_url: None,
    };

    let mut batch = ObjUrlBatch::from_urls(
        HashMap::from([(String::from("object-1"), urls)]),
        &HashMap::from([("object-1", 2), ("missing", 1)]),
    );

    assert!(batch.urls.contains_key("object-1"));

    assert!(!batch.urls.contains_key("missing"));

    let (origin, thumbnail) = batch.take("object-1").unwrap();

    assert!(origin.is_none());

    assert!(thumbnail.is_none());

    assert!(batch.urls.contains_key("object-1"));

    assert!(batch.take("object-1").is_some());

    assert!(!batch.urls.contains_key("object-1"));

    assert!(batch.take("missing").is_none());
}
