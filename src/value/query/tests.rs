//! Tests for the query-string grouping deserialisation.
//!
//! The `GroupedQuery` extractor and `from_grouped_query` function are tested
//! with both single and repeated query parameters.

use serde::Deserialize;

use crate::value::query::from_grouped_query;

/// Minimal enum that mimics incl opt patterns.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum TestOpt {
    /// Foo variant.
    Foo,
    /// Bar variant.
    Bar,
}

/// Struct that mimics handler query structs — Vec field with deserialize_with.
#[derive(Debug, Deserialize)]
struct TestQuery {
    #[serde(
        default,
        rename = "incl",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    incl_opt: Vec<TestOpt>,

    offset: u32,
    limit: u32,
}

/// Struct that mimics ComicListQuery — two Vec fields.
#[derive(Debug, Deserialize)]
struct TestMultiQuery {
    #[serde(
        default,
        rename = "incl",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    incl_opt: Vec<TestOpt>,

    #[serde(
        default,
        rename = "with",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    with_opt: Vec<TestOpt>,

    offset: u32,
    limit: u32,
}

#[test]
fn grouped_repeated_incl() {
    //
    let qs = "incl=foo&incl=bar&offset=0&limit=20";

    let parsed: TestQuery = from_grouped_query(qs).unwrap();

    assert_eq!(parsed.incl_opt, vec![TestOpt::Foo, TestOpt::Bar]);

    assert_eq!(parsed.offset, 0);

    assert_eq!(parsed.limit, 20);
}

#[test]
fn grouped_repeated_with() {
    //
    let qs = "incl=foo&with=foo&with=bar&offset=0&limit=20";

    let parsed: TestMultiQuery = from_grouped_query(qs).unwrap();

    assert_eq!(parsed.incl_opt, vec![TestOpt::Foo]);

    assert_eq!(parsed.with_opt, vec![TestOpt::Foo, TestOpt::Bar]);
}

#[test]
fn grouped_both_incl_with_repeated() {
    //
    let qs = "incl=foo&incl=bar&with=foo&with=bar&offset=0&limit=20";

    let parsed: TestMultiQuery = from_grouped_query(qs).unwrap();

    assert_eq!(parsed.incl_opt, vec![TestOpt::Foo, TestOpt::Bar]);

    assert_eq!(parsed.with_opt, vec![TestOpt::Foo, TestOpt::Bar]);
}

#[test]
fn grouped_empty_incl_and_with() {
    //
    let qs = "offset=10&limit=5";

    let parsed: TestMultiQuery = from_grouped_query(qs).unwrap();

    assert!(parsed.incl_opt.is_empty());

    assert!(parsed.with_opt.is_empty());

    assert_eq!(parsed.offset, 10);

    assert_eq!(parsed.limit, 5);
}

#[test]
fn grouped_single_incl() {
    //
    let qs = "incl=foo&offset=0&limit=20";

    let parsed: TestQuery = from_grouped_query(qs).unwrap();

    assert_eq!(parsed.incl_opt, vec![TestOpt::Foo]);
}

#[test]
fn grouped_single_with() {
    //
    let qs = "with=bar&offset=0&limit=20";

    let parsed: TestMultiQuery = from_grouped_query(qs).unwrap();

    assert_eq!(parsed.with_opt, vec![TestOpt::Bar]);
}

#[test]
fn grouped_multi_without_deserialize_with() {
    // Vec<String> without deserialize_with — multiple repeated values
    // produce a JSON array which Vec<String> handles natively.
    //
    #[derive(Debug, Deserialize)]
    struct PlainQuery {
        #[serde(default, rename = "tags")]
        tags: Vec<String>,
    }

    let qs = "tags=a&tags=b&tags=c";

    let parsed: PlainQuery = from_grouped_query(qs).unwrap();

    assert_eq!(
        parsed.tags,
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
}

#[test]
fn grouped_url_encoded_values() {
    //
    #[derive(Debug, Deserialize)]
    struct Test {
        #[serde(default, rename = "q")]
        q: String,
    }

    let parsed: Test = from_grouped_query("q=hello%20world").unwrap();

    assert_eq!(parsed.q, "hello world");
}
